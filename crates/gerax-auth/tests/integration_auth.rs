use std::sync::Arc;

use gerax_auth::{
    TokenStorage,
    login, refresh, AuthMiddleware, AuthState, JwtAuthenticator, MemoryTokenStorage,
};
use gerax_http::middleware::{Middleware, Next};
use gerax_http::routing::{Context, HttpMethod, Request, Response, Handler};
use gerax_http::ServerResult;

/// Testa o fluxo completo de autenticação:
/// 1. Login com credenciais válidas retorna token pair
/// 2. Acesso a rota protegida sem token é negado
/// 3. Acesso a rota protegida com token válido é permitido
/// 4. Acesso a rota protegida com scope insuficiente é negado
/// 5. Refresh de token funciona e rotaciona o antigo
/// 6. Reutilização de refresh token antigo é negada
#[tokio::test]
async fn full_auth_flow() {
    // Setup state que implementa AuthState
    struct AppState {
        jwt: JwtAuthenticator,
        store: Arc<dyn TokenStorage>,
    }

    impl AuthState for AppState {
        fn jwt(&self) -> &JwtAuthenticator {
            &self.jwt
        }
        fn token_storage(&self) -> &dyn TokenStorage {
            self.store.as_ref()
        }
    }

    let state = Arc::new(AppState {
        jwt: JwtAuthenticator::hs256("secret", 0),
        store: Arc::new(MemoryTokenStorage::new()),
    });

    // 1) Login com credenciais válidas
    let login_result = login(|_ctx| async move { Ok(sample_claims()) })
        .call(Context::new(Arc::clone(&state), login_request()))
        .await
        .unwrap();

    let token_pair: gerax_auth::TokenPair =
        serde_json::from_slice(&login_result.body).unwrap();
    assert!(!token_pair.access_token.is_empty());
    assert!(!token_pair.refresh_token.is_empty());

    // 2) Acesso sem token é negado pelo middleware
    let auth_middleware = AuthMiddleware::new(
        state.jwt.clone(),
        None::<MockAuthorizer>,
        vec!["/auth/login".into()],
    );

    let protected_ctx = Context::new(Arc::clone(&state), protected_request(None));
    let result = auth_middleware
        .handle(protected_ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
        .await;

    assert!(result.is_err(), "acesso sem token deve ser negado");

    // 3) Acesso com token válido é permitido
    let auth_ctx = Context::new(
        Arc::clone(&state),
        protected_request(Some(&token_pair.access_token)),
    );
    let result = auth_middleware
        .handle(auth_ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
        .await;

    assert!(result.is_ok(), "acesso com token válido deve ser permitido");

    // 4) Scope insuficiente é negado
    let authorizer = MockAuthorizer;
    let auth_middleware_scoped = AuthMiddleware::new(
        state.jwt.clone(),
        Some(authorizer),
        vec![],
    )
    .with_scope_resolver(|_| vec!["admin".to_string()]);

    let scoped_ctx = Context::new(
        Arc::clone(&state),
        protected_request(Some(&token_pair.access_token)),
    );
    let result = auth_middleware_scoped
        .handle(scoped_ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
        .await;

    assert!(result.is_err(), "scope insuficiente deve ser negado");

    // 5) Refresh de token funciona
    let refresh_result = refresh(Context::new(
        Arc::clone(&state),
        refresh_request(&token_pair.refresh_token),
    ))
    .await
    .unwrap();

    let refreshed: gerax_auth::TokenPair =
        serde_json::from_slice(&refresh_result.body).unwrap();
    assert!(!refreshed.refresh_token.is_empty());

    // 6) Reutilização do refresh token antigo é negada
    let reuse_result = refresh(Context::new(
        Arc::clone(&state),
        refresh_request(&token_pair.refresh_token),
    ))
    .await;

    assert!(reuse_result.is_err(), "refresh token reutilizado deve ser negado");
}

#[tokio::test]
async fn login_with_invalid_credentials_returns_error() {
    struct AppState {
        jwt: JwtAuthenticator,
    }

    impl AuthState for AppState {
        fn jwt(&self) -> &JwtAuthenticator {
            &self.jwt
        }
        fn token_storage(&self) -> &dyn TokenStorage {
            unimplemented!()
        }
    }

    let state = Arc::new(AppState {
        jwt: JwtAuthenticator::hs256("secret", 0),
    });

    let result = login(|_ctx| async move { Err(gerax_auth::AuthError::InvalidCredentials) })
        .call(Context::new(Arc::clone(&state), login_request()))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn middleware_blocks_expired_token() {
    struct AppState {
        jwt: JwtAuthenticator,
    }

    impl AuthState for AppState {
        fn jwt(&self) -> &JwtAuthenticator {
            &self.jwt
        }
        fn token_storage(&self) -> &dyn TokenStorage {
            unimplemented!()
        }
    }

    let jwt = JwtAuthenticator::hs256("secret", 0);
    let state = Arc::new(AppState { jwt: jwt.clone() });

    let expired_token = {
        let claims = gerax_auth::Claims {
            sub: "user-123".into(),
            exp: 1, // expirado
            iat: 0,
            scope: vec![],
        };
        jwt.encode_token(&claims).unwrap()
    };

    let auth_middleware = AuthMiddleware::new(
        state.jwt.clone(),
        None::<MockAuthorizer>,
        vec![],
    );

    let mut request = Request::new(HttpMethod::Get, "/protected".into(), Vec::new());
    request.headers.insert("authorization", format!("Bearer {}", expired_token));
    let ctx = Context::new(Arc::clone(&state), request);

    let result = auth_middleware
        .handle(ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
        .await;

    assert!(result.is_err(), "token expirado deve ser negado");
}

// Helpers

fn sample_claims() -> gerax_auth::Claims {
    gerax_auth::Claims {
        sub: "user-123".into(),
        exp: u64::MAX,
        iat: 0,
        scope: vec!["read".into()],
    }
}

fn login_request() -> Request {
    Request::new(HttpMethod::Post, "/auth/login".into(), Vec::new())
}

fn protected_request(token: Option<&str>) -> Request {
    let mut request = Request::new(HttpMethod::Get, "/api/protected".into(), Vec::new());
    if let Some(token) = token {
        request.headers.insert("authorization", format!("Bearer {}", token));
    }
    request
}

fn refresh_request(refresh_token: &str) -> Request {
    let body = serde_json::to_vec(&serde_json::json!({ "refresh_token": refresh_token })).unwrap();
    Request::new(HttpMethod::Post, "/auth/refresh".into(), body)
}

async fn next_ok<State>(_ctx: Context<State>) -> ServerResult<Response> {
    Ok(gerax_http::routing::Response::ok("ok"))
}

struct MockAuthorizer;

#[async_trait::async_trait]
impl<State> gerax_auth::Authorizer<State> for MockAuthorizer {
    async fn authorize(&self, _ctx: &Context<State>, required: &[String]) -> gerax_auth::AuthResult<bool> {
        Ok(required.contains(&"read".to_string()))
    }
}
