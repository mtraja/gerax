use std::future::Future;
use std::sync::Arc;

use gerax_http::routing::{Context, Response};
use gerax_http::ServerResult;
use gerax_http::routing::Handler;

use crate::traits::{AuthError, AuthResult};
use crate::types::{Claims, RefreshToken, TokenPair};
use crate::{JwtAuthenticator, MemoryTokenStorage, TokenStorage};

/// Handler para rota de login (`POST /auth/login`).
///
/// Espera um JSON no corpo com credenciais. A validação das credenciais é
/// delegada à closure `validate_credentials`, que retorna `Claims` em caso de
/// sucesso. Em caso de sucesso, o handler gera um `access_token` (JWT) e um
/// `refresh_token`, persiste o refresh token e retorna um `TokenPair`.
///
/// ```rust
/// use std::sync::Arc;
///
/// use gerax_auth::{Claims, JwtAuthenticator, LoginHandler, MemoryTokenStorage};
/// use gerax_http::routing::{Context, Router};
///
/// struct AppState;
///
/// let router: Router<AppState> = Router::new();
/// let validate = |ctx: Context<AppState>| async move {
///     Ok(Claims {
///         sub: "user-123".into(),
///         exp: u64::MAX,
///         iat: 0,
///         scope: vec![],
///     })
/// };
///
/// router.post("/auth/login", LoginHandler::new(
///     validate,
///     JwtAuthenticator::hs256("secret", 30),
///     Arc::new(MemoryTokenStorage::new()),
/// ));
/// ```
pub struct LoginHandler<State, V, Fut>
where
    V: Fn(Context<State>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AuthResult<Claims>> + Send + 'static,
    State: Send + Sync + 'static,
{
    validate_credentials: V,
    jwt_authenticator: JwtAuthenticator,
    refresh_store: Arc<dyn TokenStorage>,
    _marker: std::marker::PhantomData<fn() -> State>,
}

impl<State, V, Fut> LoginHandler<State, V, Fut>
where
    V: Fn(Context<State>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AuthResult<Claims>> + Send + 'static,
    State: Send + Sync + 'static,
{
    pub fn new(
        validate_credentials: V,
        jwt_authenticator: JwtAuthenticator,
        refresh_store: Arc<dyn TokenStorage>,
    ) -> Self {
        Self {
            validate_credentials,
            jwt_authenticator,
            refresh_store,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<State, V, Fut> Handler<State> for LoginHandler<State, V, Fut>
where
    V: Fn(Context<State>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = AuthResult<Claims>> + Send + 'static,
    State: Send + Sync + 'static,
{
    fn call<'life0, 'async_trait>(
        &'life0 self,
        ctx: Context<State>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = ServerResult<Response>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let claims = (self.validate_credentials)(ctx.clone()).await?;

            let access_token = self.jwt_authenticator.encode_token(&claims)?;
            let refresh_token = generate_refresh_token(&claims);

            self.refresh_store
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
            })
        })
    }
}

/// Handler para rota de refresh (`POST /auth/refresh`).
///
/// Espera um JSON no corpo com `refresh_token`. Valida o token, rotaciona e
/// retorna um novo `TokenPair` com `access_token` e `refresh_token` renovados.
///
/// ```rust
/// use std::sync::Arc;
///
/// use gerax_auth::{JwtAuthenticator, MemoryTokenStorage, RefreshHandler};
/// use gerax_http::routing::{Context, Router};
///
/// struct AppState;
///
/// let router: Router<AppState> = Router::new();
/// router.post("/auth/refresh", RefreshHandler::new(
///     JwtAuthenticator::hs256("secret", 30),
///     Arc::new(MemoryTokenStorage::new()),
/// ));
/// ```
pub struct RefreshHandler<State>
where
    State: Send + Sync + 'static,
{
    jwt_authenticator: JwtAuthenticator,
    refresh_store: Arc<dyn TokenStorage>,
    _marker: std::marker::PhantomData<fn() -> State>,
}

impl<State> RefreshHandler<State>
where
    State: Send + Sync + 'static,
{
    pub fn new(
        jwt_authenticator: JwtAuthenticator,
        refresh_store: Arc<dyn TokenStorage>,
    ) -> Self {
        Self {
            jwt_authenticator,
            refresh_store,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<State> Handler<State> for RefreshHandler<State>
where
    State: Send + Sync + 'static,
{
    fn call<'life0, 'async_trait>(
        &'life0 self,
        ctx: Context<State>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = ServerResult<Response>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            #[derive(serde::Deserialize)]
            struct RefreshRequest {
                refresh_token: String,
            }

            let request: RefreshRequest = serde_json::from_slice(&ctx.request().body)
                .map_err(|e| AuthError::Internal(format!("falha ao ler refresh request: {e}")))?;

            let stored = self
                .refresh_store
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

            let access_token = self.jwt_authenticator.encode_token(&claims)?;
            let new_refresh_token = generate_refresh_token(&claims);

            let mut rotated = stored;
            rotated.rotated = true;
            self.refresh_store.save(rotated).await?;

            self.refresh_store
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
            })
        })
    }
}

fn generate_refresh_token(claims: &Claims) -> String {
    format!("rt-{}-{}", claims.sub, uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Claims;

    fn sample_claims() -> Claims {
        Claims {
            sub: "user-123".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into()],
        }
    }

    fn test_state() -> JwtAuthenticator {
        JwtAuthenticator::hs256("secret", 0)
    }

    #[tokio::test]
    async fn login_handler_returns_token_pair() {
        let authenticator = test_state();
        let store = Arc::new(crate::MemoryTokenStorage::new());

        let handler = LoginHandler::new(
            |_ctx| async move { Ok(sample_claims()) },
            authenticator,
            store,
        );

        let request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Post,
            "/auth/login".into(),
            Vec::new(),
        );
        let ctx = Context::new(std::sync::Arc::new(()), request);

        let result = handler.call(ctx).await.unwrap();
        let pair: TokenPair = serde_json::from_slice(&result.body).unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert!(pair.refresh_token.starts_with("rt-user-123-"));
    }

    #[tokio::test]
    async fn refresh_handler_returns_new_tokens() {
        let authenticator = test_state();
        let store = Arc::new(crate::MemoryTokenStorage::new());

        let claims = sample_claims();
        let refresh_token = generate_refresh_token(&claims);

        store
            .save(RefreshToken {
                token: refresh_token.clone(),
                user_id: claims.sub.clone(),
                expires_at: u64::MAX,
                rotated: false,
            })
            .await
            .unwrap();

        let handler = RefreshHandler::new(authenticator, store.clone());

        let body = serde_json::to_vec(&serde_json::json!({ "refresh_token": refresh_token })).unwrap();
        let request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Post,
            "/auth/refresh".into(),
            body,
        );
        let ctx = Context::new(std::sync::Arc::new(()), request);

        let result = handler.call(ctx).await.unwrap();
        let pair: TokenPair = serde_json::from_slice(&result.body).unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
    }
}
