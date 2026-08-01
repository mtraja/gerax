use gerax_http::routing::{Context, ExtractError};
use gerax_http::routing::FromContext;

use crate::types::Claims;

/// Usuário autenticado extraído das `Claims` injetadas pelo `AuthMiddleware`.
///
/// Uso em handlers:
///
/// ```rust
/// use gerax_auth::AuthenticatedUser;
/// use gerax_http::routing::{Context, Response};
///
/// struct AppState;
///
/// async fn protected_handler(
///     ctx: Context<AppState>,
///     user: AuthenticatedUser,
/// ) -> Response {
///     println!("User: {}", user.0.sub);
///     Response::ok("Secret data")
/// }
/// ```
pub struct AuthenticatedUser(pub Claims);

impl<State> FromContext<State> for AuthenticatedUser
where
    State: Send + Sync + 'static,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<State>) -> Result<Self, Self::Rejection> {
        ctx.extensions()
            .get::<Claims>()
            .map(|c| AuthenticatedUser(c.as_ref().clone()))
            .ok_or_else(|| ExtractError::Missing("authenticated user".into()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::types::Claims;

    fn build_context_with_claims(claims: Claims) -> Context<()> {
        let request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Get,
            "/".into(),
            Vec::new(),
        );
        let mut ctx = Context::new(Arc::new(()), request);
        ctx.extensions_mut().insert(claims);
        ctx
    }

    #[test]
    fn extractor_returns_user_when_claims_present() {
        let claims = Claims {
            sub: "user-123".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into()],
        };
        let ctx = build_context_with_claims(claims.clone());

        let user = AuthenticatedUser::from_context(&ctx).unwrap();
        assert_eq!(user.0.sub, "user-123");
        assert_eq!(user.0.scope, claims.scope);
    }

    #[test]
    fn extractor_returns_missing_when_no_claims() {
        let request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Get,
            "/".into(),
            Vec::new(),
        );
        let ctx = Context::new(Arc::new(()), request);

        let result = AuthenticatedUser::from_context(&ctx);
        assert!(result.is_err());
    }
}
