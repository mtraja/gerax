use crate::GraphqlError;
use gerax_auth::Claims;

/// Resolved auth claims from the GraphQL context.
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
        context
            .extensions()
            .get::<Claims>()
            .map(|arc| (*arc).clone())
            .ok_or_else(|| GraphqlError::Unauthorized("claims not found in context".to_string()))
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
