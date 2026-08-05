use gerax_http::routing::context::Context as HttpContext;
use serde_json::Value;

/// Contexto GraphQL que reutiliza o `Context<State>` do `gerax-http`.
///
/// Nunca criar um segundo contexto. Este módulo apenas
/// fornece extensões específicas para GraphQL quando necessário.
pub type GraphqlContext<State> = HttpContext<State>;

/// Extensões específicas para GraphQL no contexto.
pub struct GraphqlExtensions {
    /// Informações do usuário autenticado.
    pub auth_claims: Option<Value>,
    /// Indica se a introspection está habilitada.
    pub introspection_enabled: bool,
    /// Limite de complexidade configurado.
    pub complexity_limit: Option<usize>,
    /// Limite de profundidade configurado.
    pub depth_limit: Option<usize>,
}

impl GraphqlExtensions {
    /// Cria novas extensões GraphQL com valores padrão.
    pub fn new() -> Self {
        Self {
            auth_claims: None,
            introspection_enabled: true,
            complexity_limit: None,
            depth_limit: None,
        }
    }

    /// Define as claims de autenticação nas extensões.
    pub fn with_auth_claims(mut self, claims: Value) -> Self {
        self.auth_claims = Some(claims);
        self
    }

    /// Habilita ou desabilita a introspection.
    pub fn with_introspection(mut self, enabled: bool) -> Self {
        self.introspection_enabled = enabled;
        self
    }

    /// Define o limite de complexidade.
    pub fn with_complexity_limit(mut self, limit: usize) -> Self {
        self.complexity_limit = Some(limit);
        self
    }

    /// Define o limite de profundidade.
    pub fn with_depth_limit(mut self, limit: usize) -> Self {
        self.depth_limit = Some(limit);
        self
    }
}

impl Default for GraphqlExtensions {
    fn default() -> Self {
        Self::new()
    }
}
