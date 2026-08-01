use std::sync::Arc;

use gerax_http::middleware::{Middleware, Next};
use gerax_http::routing::{Context, Response};
use gerax_http::ServerResult;

use crate::traits::{Authenticator, Authorizer, AuthError};

/// Middleware de autenticação/autorização plugável em qualquer adapter `gerax-http`.
///
/// - Aplica o `Authenticator` em rotas protegidas.
/// - Opcionalmente aplica o `Authorizer` quando fornecido.
/// - Libera acesso imediato para `public_paths`.
/// - Injeta as `Claims` autenticadas em `Context::extensions` para uso downstream.
pub struct AuthMiddleware<A, Z> {
    authenticator: Arc<A>,
    authorizer: Option<Arc<Z>>,
    public_paths: Vec<String>,
    scope_resolver: Option<Arc<dyn Fn(&str) -> Vec<String> + Send + Sync + 'static>>,
}

impl<A, Z> AuthMiddleware<A, Z> {
    pub fn new(authenticator: A, authorizer: Option<Z>, public_paths: Vec<String>) -> Self {
        Self {
            authenticator: Arc::new(authenticator),
            authorizer: authorizer.map(Arc::new),
            public_paths,
            scope_resolver: None,
        }
    }

    /// Define uma closure customizada para mapear paths para scopes exigidos.
    ///
    /// Se não for chamado, o `Authorizer` não será acionado (comportamento padrão).
    pub fn with_scope_resolver<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&str) -> Vec<String> + Send + Sync + 'static,
    {
        self.scope_resolver = Some(Arc::new(resolver));
        self
    }
}

impl<State, A, Z> Middleware<State> for AuthMiddleware<A, Z>
where
    A: Authenticator<State> + 'static,
    Z: Authorizer<State> + 'static,
    State: Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "auth"
    }

    fn handle<'life0, 'async_trait>(
        &'life0 self,
        ctx: Context<State>,
        next: Next<State>,
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
            let path = ctx.request().path().to_string();

            // 1) Paths públicos passam direto.
            if self.public_paths.iter().any(|p| path.starts_with(p)) {
                return next.call(ctx).await;
            }

            // 2) Extrair token e autenticar.
            let claims = self
                .authenticator
                .authenticate(&ctx)
                .await?
                .ok_or(AuthError::MissingToken)?;

            // 3) Autorizar se houver Authorizer configurado e resolver de scopes.
            if let Some(authorizer) = &self.authorizer {
                if let Some(resolver) = &self.scope_resolver {
                    let required_scope = resolver(&path);
                    if !authorizer.authorize(&ctx, &required_scope).await? {
                        return Err(AuthError::Forbidden.into());
                    }
                }
            }

            // 4) Armazenar claims no contexto para handlers downstream.
            let mut ctx = ctx;
            ctx.extensions_mut().insert(claims);

            next.call(ctx).await
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::AuthResult;
    use crate::jwt::JwtAuthenticator;
    use crate::traits::Authorizer;
    use crate::types::Claims;

    struct MockAuthenticator;

    #[async_trait::async_trait]
    impl<State> Authenticator<State> for MockAuthenticator {
        async fn authenticate(&self, _ctx: &Context<State>) -> AuthResult<Option<Claims>> {
            Ok(Some(Claims {
                sub: "user-123".into(),
                exp: u64::MAX,
                iat: 0,
                scope: vec!["read".into()],
            }))
        }
    }

    struct MockAuthorizer;

    #[async_trait::async_trait]
    impl<State> Authorizer<State> for MockAuthorizer {
        async fn authorize(&self, _ctx: &Context<State>, required: &[String]) -> AuthResult<bool> {
            Ok(required.contains(&"read".to_string()))
        }
    }

    fn build_context(path: &str) -> Context<()> {
        let request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Get,
            path.into(),
            Vec::new(),
        );
        Context::new(Arc::new(()), request)
    }

    async fn next_ok(_ctx: Context<()>) -> ServerResult<Response> {
        Ok(gerax_http::routing::Response::ok("ok"))
    }

    #[tokio::test]
    async fn middleware_blocks_unauthenticated() {
        let authenticator = JwtAuthenticator::hs256("secret", 0);
        let middleware = AuthMiddleware::new(authenticator, None::<MockAuthorizer>, vec![]);
        let ctx = build_context("/protected");

        let result = middleware
            .handle(ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn middleware_allows_public_path() {
        let authenticator = JwtAuthenticator::hs256("secret", 0);
        let middleware = AuthMiddleware::new(authenticator, None::<MockAuthorizer>, vec!["/public".into()]);
        let ctx = build_context("/public/page");

        let result = middleware
            .handle(ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn middleware_denies_insufficient_scope() {
        let authenticator = MockAuthenticator;
        let authorizer = MockAuthorizer;
        let middleware = AuthMiddleware::new(authenticator, Some(authorizer), vec![])
            .with_scope_resolver(|_| vec!["admin".to_string()]);
        let ctx = build_context("/admin");

        let result = middleware
            .handle(ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn middleware_allows_when_scope_matches() {
        let authenticator = MockAuthenticator;
        let authorizer = MockAuthorizer;
        let middleware = AuthMiddleware::new(authenticator, Some(authorizer), vec![])
            .with_scope_resolver(|_| vec!["read".to_string()]);
        let ctx = build_context("/api/read");

        let result = middleware
            .handle(ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn middleware_skips_authorization_without_resolver() {
        let authenticator = MockAuthenticator;
        let authorizer = MockAuthorizer;
        let middleware = AuthMiddleware::new(authenticator, Some(authorizer), vec![]);
        let ctx = build_context("/any");

        let result = middleware
            .handle(ctx, Next::new(|ctx| Box::pin(next_ok(ctx))))
            .await;

        assert!(result.is_ok());
    }
}
