

use crate::traits::{AuthError, AuthResult, Authorizer};
use crate::types::Claims;
use gerax_http::routing::Context;

/// Authorizer baseado em scopes/roles presentes nas `Claims`.
///
/// Verifica se o usuário autenticado possui pelo menos um dos scopes exigidos.
/// Se `strict = true`, exige **todos** os scopes. Se `false`, exige pelo menos um.
#[derive(Debug, Clone)]
pub struct ScopeAuthorizer {
    strict: bool,
}

impl Default for ScopeAuthorizer {
    fn default() -> Self {
        Self::new(false)
    }
}

impl ScopeAuthorizer {
    pub fn new(strict: bool) -> Self {
        Self { strict }
    }

    pub fn any() -> Self {
        Self::new(false)
    }

    pub fn all() -> Self {
        Self::new(true)
    }
}

#[async_trait::async_trait]
impl<State> Authorizer<State> for ScopeAuthorizer
where
    State: Send + Sync + 'static,
{
    async fn authorize(&self, ctx: &Context<State>, required: &[String]) -> AuthResult<bool> {
        let claims = ctx
            .extensions()
            .get::<Claims>()
            .ok_or(AuthError::MissingToken)?;

        if required.is_empty() {
            return Ok(true);
        }

        if self.strict {
            Ok(required.iter().all(|scope| claims.scope.contains(scope)))
        } else {
            Ok(required.iter().any(|scope| claims.scope.contains(scope)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Claims;

    #[derive(Debug, Clone)]
    struct MockState {
        _claims: Claims,
    }

    fn context_with_claims(claims: Claims) -> Context<MockState> {
        let request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Get,
            "/".into(),
            Vec::new(),
        );
        let mut ctx = Context::new(std::sync::Arc::new(MockState { _claims: claims.clone() }), request);
        ctx.extensions_mut().insert(claims.clone());
        ctx
    }

    #[tokio::test]
    async fn authorizer_allows_when_any_scope_matches() {
        let authorizer = ScopeAuthorizer::any();
        let claims = Claims {
            sub: "user".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into(), "write".into()],
        };
        let ctx = context_with_claims(claims);

        assert!(authorizer.authorize(&ctx, &["read".into()]).await.unwrap());
        assert!(authorizer.authorize(&ctx, &["write".into()]).await.unwrap());
        assert!(authorizer.authorize(&ctx, &["read".into(), "admin".into()]).await.unwrap());
    }

    #[tokio::test]
    async fn authorizer_denies_when_no_scope_matches() {
        let authorizer = ScopeAuthorizer::any();
        let claims = Claims {
            sub: "user".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into()],
        };
        let ctx = context_with_claims(claims);

        assert!(!authorizer.authorize(&ctx, &["admin".into()]).await.unwrap());
    }

    #[tokio::test]
    async fn authorizer_strict_requires_all_scopes() {
        let authorizer = ScopeAuthorizer::all();
        let claims = Claims {
            sub: "user".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into(), "write".into()],
        };
        let ctx = context_with_claims(claims);

        assert!(authorizer.authorize(&ctx, &["read".into(), "write".into()]).await.unwrap());
        assert!(!authorizer.authorize(&ctx, &["read".into(), "admin".into()]).await.unwrap());
    }
}
