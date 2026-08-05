use crate::{GraphqlContext, GraphqlError};
use gerax_auth::Claims;

/// Extensão para acessar as credenciais inseridas pelo `gerax_auth::AuthMiddleware`.
///
/// Importe este trait para recuperar um valor autenticado diretamente do contexto:
///
/// ```ignore
/// use gerax_auth::Claims;
/// use gerax_graphql::GraphqlAuthContext;
///
/// let claims = ctx.auth::<Claims>()?;
/// ```
pub trait GraphqlAuthContext {
    /// Obtém um valor autenticado armazenado nas extensões do contexto.
    ///
    /// Retorna [`GraphqlError::Unauthorized`] quando o valor não foi inserido
    /// pelo middleware de autenticação.
    fn auth<T>(&self) -> Result<T, GraphqlError>
    where
        T: Clone + Send + Sync + 'static;
}

impl<State> GraphqlAuthContext for GraphqlContext<State>
where
    State: Send + Sync,
{
    fn auth<T>(&self) -> Result<T, GraphqlError>
    where
        T: Clone + Send + Sync + 'static,
    {
        self.extensions()
            .get::<T>()
            .map(|value| (*value).clone())
            .ok_or_else(|| GraphqlError::Unauthorized("authentication is required".to_string()))
    }
}

/// Resolved auth claims from the GraphQL context.
#[derive(Debug, Clone)]
pub struct AuthClaims {
    /// The authenticated user's claims.
    pub claims: Claims,
}

impl AuthClaims {
    /// Cria novas claims a partir das claims do gerax-auth.
    pub fn new(claims: Claims) -> Self {
        Self { claims }
    }

    /// Retorna o subject (identidade) do claim.
    pub fn subject(&self) -> &str {
        &self.claims.sub
    }

    /// Retorna os escopos/roles do claim.
    pub fn scopes(&self) -> &[String] {
        &self.claims.scope
    }

    /// Verifica se o usuário possui o escopo solicitado.
    pub fn has_scope(&self, scope: &str) -> bool {
        self.claims.scope.iter().any(|candidate| candidate == scope)
    }
}

/// Extractor de autenticação para GraphQL.
///
/// Permite extrair claims do contexto GraphQL
/// de forma integrada com gerax-auth.
pub struct AuthExtractor;

impl AuthExtractor {
    /// Cria um novo extractor de autenticação.
    pub fn new() -> Self {
        Self
    }

    /// Extrai as claims do contexto GraphQL.
    pub fn extract_claims(
        &self,
        context: &crate::GraphqlContext<impl Send + Sync>,
    ) -> Result<Claims, GraphqlError> {
        context.auth::<Claims>()
    }
}

/// Integração de auth com o contexto GraphQL.
///
/// Fornece métodos convenientes para acessar
/// as claims e o usuário autenticado.
pub struct AuthIntegration;

impl AuthIntegration {
    /// Cria uma nova integração de auth.
    pub fn new() -> Self {
        Self
    }

    /// Adiciona as claims ao contexto GraphQL.
    pub fn with_claims(
        &self,
        context: &mut crate::GraphqlContext<impl Send + Sync>,
        claims: &Claims,
    ) {
        context.extensions_mut().insert(claims.clone());
    }

    /// Extrai as claims do contexto.
    pub fn claims_from_context(
        &self,
        context: &crate::GraphqlContext<impl Send + Sync>,
    ) -> Result<Claims, GraphqlError> {
        AuthExtractor::new().extract_claims(context)
    }
}

impl Default for AuthExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AuthIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gerax_http::routing::{Context, HttpMethod, Request};

    use super::{AuthClaims, AuthExtractor, AuthIntegration, GraphqlAuthContext};
    use crate::GraphqlError;
    use gerax_auth::Claims;

    fn context() -> Context<()> {
        Context::new(
            Arc::new(()),
            Request::new(HttpMethod::Post, "/graphql".to_string(), Vec::new()),
        )
    }

    fn claims() -> Claims {
        Claims {
            sub: "user-42".to_string(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["posts:read".to_string()],
        }
    }

    #[test]
    fn retrieves_claims_inserted_by_auth_middleware() {
        let context = context();
        let claims = claims();
        context.extensions().insert(claims.clone());

        let resolved = context.auth::<Claims>();

        assert_eq!(resolved.map(|value| value.sub), Ok(claims.sub));
    }

    #[test]
    fn returns_unauthorized_when_claims_are_absent() {
        let result = context().auth::<Claims>();

        assert!(matches!(result, Err(GraphqlError::Unauthorized(_))));
    }

    #[test]
    fn integration_and_extractor_share_the_context_contract() {
        let mut context = context();
        let claims = claims();
        AuthIntegration::new().with_claims(&mut context, &claims);

        let extracted = AuthExtractor::new().extract_claims(&context);

        assert_eq!(extracted.map(|value| value.sub), Ok(claims.sub));
    }

    #[test]
    fn auth_claims_checks_scopes() {
        let claims = AuthClaims::new(claims());

        assert!(claims.has_scope("posts:read"));
        assert!(!claims.has_scope("posts:write"));
    }
}
