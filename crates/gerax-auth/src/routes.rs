use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use gerax_http::routing::{Context, Response};
use gerax_http::routing::Handler;
use gerax_http::ServerResult;

use crate::traits::{AuthError, AuthResult};
use crate::types::{Claims, RefreshToken, TokenPair};
use crate::{JwtAuthenticator, TokenStorage};

/// Contrato de state que fornece dependências de autenticação.
///
/// Implemente esta trait no `State` da sua aplicação para usar os handlers
/// `login` e `refresh` diretamente no `Router`.
pub trait AuthState: Send + Sync + 'static {
    /// Autenticador JWT configurado.
    fn jwt(&self) -> &JwtAuthenticator;
    /// Backend de persistência de refresh tokens.
    fn token_storage(&self) -> &dyn TokenStorage;
}

/// Handler de login (`POST /auth/login`).
///
/// Espera JSON no corpo com credenciais. A validação é delegada à closure
/// `validate_credentials`, que retorna `Claims` em caso de sucesso.
/// Gera e retorna um `TokenPair` (access_token + refresh_token).
pub fn login<State, V, Fut>(
    validate_credentials: V,
) -> impl Handler<State>
where
    State: AuthState,
    V: Fn(Context<State>) -> Fut + Send + Sync + 'static + Clone,
    Fut: Future<Output = AuthResult<Claims>> + Send + 'static,
{
    let validator = Arc::new(validate_credentials);
    move |ctx: Context<State>| {
        let v = validator.clone();
        Box::pin(async move { login_impl(ctx, v).await })
            as Pin<Box<dyn Future<Output = ServerResult<Response>> + Send>>
    }
}

async fn login_impl<State, V, Fut>(
    ctx: Context<State>,
    validate_credentials: Arc<V>,
) -> ServerResult<Response>
where
    State: AuthState,
    V: Fn(Context<State>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AuthResult<Claims>> + Send + 'static,
{
    let claims = validate_credentials(ctx.clone()).await?;

    let access_token = ctx.state().jwt().encode_token(&claims)?;
    let refresh_token = generate_refresh_token(&claims);

    ctx.state().token_storage()
        .save(RefreshToken {
            token: refresh_token.clone(),
            user_id: claims.sub.clone(),
            expires_at: u64::MAX,
            rotated: false,
        })
        .await?;

    let token_pair = TokenPair {
        access_token,
        refresh_token,
    };

    let body = serde_json::to_vec(&token_pair)
        .map_err(|e| AuthError::Internal(e.to_string()))?;

    Ok(Response {
        status: 200,
        body,
        ..Default::default()
    })
}

/// Handler de refresh (`POST /auth/refresh`).
///
/// Espera JSON no corpo com `refresh_token`. Valida, rotaciona e retorna
/// um novo `TokenPair`.
pub async fn refresh<State>(ctx: Context<State>) -> ServerResult<Response>
where
    State: AuthState,
{
    #[derive(serde::Deserialize)]
    struct RefreshRequest {
        refresh_token: String,
    }

    let request: RefreshRequest = serde_json::from_slice(&ctx.request().body)
        .map_err(|e| AuthError::Internal(format!("falha ao ler refresh request: {e}")))?;

    let stored = ctx
        .state()
        .token_storage()
        .find(&request.refresh_token)
        .await?
        .ok_or(AuthError::InvalidToken)?;

    if stored.rotated {
        return Err(AuthError::InvalidToken.into());
    }

    let claims = Claims {
        sub: stored.user_id.clone(),
        exp: u64::MAX,
        iat: 0,
        scope: Vec::new(),
    };

    let access_token = ctx.state().jwt().encode_token(&claims)?;
    let new_refresh_token = generate_refresh_token(&claims);

    let mut rotated = stored;
    rotated.rotated = true;
    ctx.state().token_storage().save(rotated).await?;

    ctx.state().token_storage()
        .save(RefreshToken {
            token: new_refresh_token.clone(),
            user_id: claims.sub.clone(),
            expires_at: u64::MAX,
            rotated: false,
        })
        .await?;

    let token_pair = TokenPair {
        access_token,
        refresh_token: new_refresh_token,
    };

    let body = serde_json::to_vec(&token_pair)
        .map_err(|e| AuthError::Internal(e.to_string()))?;

    Ok(Response {
        status: 200,
        body,
        ..Default::default()
    })
}

fn generate_refresh_token(claims: &Claims) -> String {
    format!("rt-{}-{}", claims.sub, uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::types::Claims;
    use crate::MemoryTokenStorage;

    #[derive(Clone)]
    struct MockAuthState {
        jwt: JwtAuthenticator,
        store: Arc<dyn TokenStorage>,
    }

    impl AuthState for MockAuthState {
        fn jwt(&self) -> &JwtAuthenticator {
            &self.jwt
        }

        fn token_storage(&self) -> &dyn TokenStorage {
            self.store.as_ref()
        }
    }

    fn sample_claims() -> Claims {
        Claims {
            sub: "user-123".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into()],
        }
    }

    fn mock_state() -> MockAuthState {
        MockAuthState {
            jwt: JwtAuthenticator::hs256("secret", 0),
            store: Arc::new(MemoryTokenStorage::new()),
        }
    }

    #[tokio::test]
    async fn login_handler_returns_token_pair() {
        let state = mock_state();

        let handler = login(|_ctx| async move { Ok(sample_claims()) });

        let result = handler
            .call(Context::new(Arc::new(state), {
                let req = gerax_http::routing::Request::new(
                    gerax_http::routing::HttpMethod::Post,
                    "/auth/login".into(),
                    Vec::new(),
                );
                req
            }))
            .await
            .unwrap();

        let pair: TokenPair = serde_json::from_slice(&result.body).unwrap();
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert!(pair.refresh_token.starts_with("rt-user-123-"));
    }

    #[tokio::test]
    async fn refresh_handler_returns_new_tokens() {
        let state = mock_state();
        let claims = sample_claims();
        let refresh_token = generate_refresh_token(&claims);

        state.store
            .save(RefreshToken {
                token: refresh_token.clone(),
                user_id: claims.sub.clone(),
                expires_at: u64::MAX,
                rotated: false,
            })
            .await
            .unwrap();

        let body =
            serde_json::to_vec(&serde_json::json!({ "refresh_token": refresh_token })).unwrap();
        let req = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Post,
            "/auth/refresh".into(),
            body,
        );
        let ctx = Context::new(Arc::new(state), req);

        let result = refresh(ctx).await.unwrap();
        let pair: TokenPair = serde_json::from_slice(&result.body).unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
    }
}
