use crate::{GraphqlAuthContext, GraphqlError, GraphqlRequest, GraphqlResponse};
use async_trait::async_trait;
use gerax_auth::Claims;
use gerax_http::routing::context::Context as HttpContext;

/// Trait de middleware específico para GraphQL.
///
/// Interface semelhante ao middleware HTTP, mas adaptada
/// para operações GraphQL. Permite interceptar requisições
/// antes e depois da execução do resolver.
///
/// ## Exemplos de uso
///
/// - Logging: registrar cada query recebida
/// - Tracing: adicionar spans de tracing
/// - Metrics: coletar métricas de execução
/// - Cache: verificar cache antes da execução
/// - Authorization: verificar permissões do usuário
/// - Persisted Queries: validar queries persistidas
#[async_trait]
pub trait GraphqlMiddleware<State>: Send + Sync + 'static {
    /// Executa o middleware antes do resolver.
    ///
    /// Retorna `Some(GraphqlRequest)` para continuar a execução
    /// ou `None` para short-circuit (ex: cache hit, auth failed).
    async fn before_execute(
        &self,
        context: &HttpContext<State>,
        request: &GraphqlRequest,
    ) -> Result<Option<GraphqlRequest>, GraphqlError>;

    /// Executa o middleware depois do resolver.
    ///
    /// Permite modificar a resposta antes de retornar ao cliente.
    async fn after_execute(
        &self,
        context: &HttpContext<State>,
        request: &GraphqlRequest,
        response: &GraphqlResponse,
    ) -> Result<GraphqlResponse, GraphqlError>;
}

/// Middleware de logging para GraphQL.
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    /// Cria um novo middleware de logging.
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<State> GraphqlMiddleware<State> for LoggingMiddleware
where
    State: Send + Sync + 'static,
{
    async fn before_execute(
        &self,
        _context: &HttpContext<State>,
        request: &GraphqlRequest,
    ) -> Result<Option<GraphqlRequest>, GraphqlError> {
        Ok(Some(request.clone()))
    }

    async fn after_execute(
        &self,
        _context: &HttpContext<State>,
        _request: &GraphqlRequest,
        response: &GraphqlResponse,
    ) -> Result<GraphqlResponse, GraphqlError> {
        Ok(response.clone())
    }
}

/// Middleware de métricas para GraphQL.
pub struct MetricsMiddleware;

impl MetricsMiddleware {
    /// Cria um novo middleware de métricas.
    pub fn new() -> Self {
        Self
    }
}

impl Default for MetricsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<State> GraphqlMiddleware<State> for MetricsMiddleware
where
    State: Send + Sync + 'static,
{
    async fn before_execute(
        &self,
        _context: &HttpContext<State>,
        _request: &GraphqlRequest,
    ) -> Result<Option<GraphqlRequest>, GraphqlError> {
        Ok(Some(_request.clone()))
    }

    async fn after_execute(
        &self,
        _context: &HttpContext<State>,
        _request: &GraphqlRequest,
        _response: &GraphqlResponse,
    ) -> Result<GraphqlResponse, GraphqlError> {
        Ok(_response.clone())
    }
}

/// Middleware de autenticação e autorização para GraphQL.
///
/// Este middleware deve ser usado após `gerax_auth::AuthMiddleware`, que
/// valida o token e adiciona [`Claims`] às extensões do contexto. Por padrão
/// uma operação só prossegue quando há claims. Use [`Self::require_scope`] para
/// exigir escopos adicionais.
pub struct AuthMiddleware {
    required_scopes: Vec<String>,
}

impl AuthMiddleware {
    /// Cria um novo middleware de autorização.
    pub fn new() -> Self {
        Self {
            required_scopes: Vec::new(),
        }
    }

    /// Exige um escopo para executar uma operação GraphQL.
    pub fn require_scope(mut self, scope: impl Into<String>) -> Self {
        self.required_scopes.push(scope.into());
        self
    }
}

#[async_trait]
impl<State> GraphqlMiddleware<State> for AuthMiddleware
where
    State: Send + Sync + 'static,
{
    async fn before_execute(
        &self,
        context: &HttpContext<State>,
        request: &GraphqlRequest,
    ) -> Result<Option<GraphqlRequest>, GraphqlError> {
        let claims = context.auth::<Claims>()?;
        let missing_scope = self
            .required_scopes
            .iter()
            .find(|scope| !claims.scope.iter().any(|candidate| candidate == *scope));

        if let Some(scope) = missing_scope {
            return Err(GraphqlError::Forbidden(format!(
                "missing required scope: {scope}"
            )));
        }

        Ok(Some(request.clone()))
    }

    async fn after_execute(
        &self,
        _context: &HttpContext<State>,
        _request: &GraphqlRequest,
        response: &GraphqlResponse,
    ) -> Result<GraphqlResponse, GraphqlError> {
        Ok(response.clone())
    }
}

impl Default for AuthMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware de cache para GraphQL.
pub struct CacheMiddleware;

impl CacheMiddleware {
    /// Cria um novo middleware de cache.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CacheMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<State> GraphqlMiddleware<State> for CacheMiddleware
where
    State: Send + Sync + 'static,
{
    async fn before_execute(
        &self,
        _context: &HttpContext<State>,
        _request: &GraphqlRequest,
    ) -> Result<Option<GraphqlRequest>, GraphqlError> {
        Ok(Some(_request.clone()))
    }

    async fn after_execute(
        &self,
        _context: &HttpContext<State>,
        _request: &GraphqlRequest,
        _response: &GraphqlResponse,
    ) -> Result<GraphqlResponse, GraphqlError> {
        Ok(_response.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gerax_auth::Claims;
    use gerax_http::routing::{Context, HttpMethod, Request};

    use super::{AuthMiddleware, GraphqlMiddleware};
    use crate::{GraphqlError, GraphqlRequest};

    fn context() -> Context<()> {
        Context::new(
            Arc::new(()),
            Request::new(HttpMethod::Post, "/graphql".to_string(), Vec::new()),
        )
    }

    fn request() -> GraphqlRequest {
        GraphqlRequest {
            query: "query { viewer { id } }".to_string(),
            ..GraphqlRequest::default()
        }
    }

    fn claims(scopes: Vec<&str>) -> Claims {
        Claims {
            sub: "user-42".to_string(),
            exp: u64::MAX,
            iat: 0,
            scope: scopes.into_iter().map(str::to_string).collect(),
        }
    }

    #[tokio::test]
    async fn auth_middleware_rejects_anonymous_operations() {
        let result = AuthMiddleware::new()
            .before_execute(&context(), &request())
            .await;

        assert!(matches!(result, Err(GraphqlError::Unauthorized(_))));
    }

    #[tokio::test]
    async fn auth_middleware_allows_authenticated_operations() {
        let context = context();
        context.extensions().insert(claims(Vec::new()));

        let result = AuthMiddleware::new()
            .before_execute(&context, &request())
            .await;

        assert!(matches!(result, Ok(Some(_))));
    }

    #[tokio::test]
    async fn auth_middleware_enforces_required_scopes() {
        let context = context();
        context.extensions().insert(claims(vec!["posts:read"]));

        let allowed = AuthMiddleware::new()
            .require_scope("posts:read")
            .before_execute(&context, &request())
            .await;
        let denied = AuthMiddleware::new()
            .require_scope("posts:write")
            .before_execute(&context, &request())
            .await;

        assert!(matches!(allowed, Ok(Some(_))));
        assert!(matches!(denied, Err(GraphqlError::Forbidden(_))));
    }
}
