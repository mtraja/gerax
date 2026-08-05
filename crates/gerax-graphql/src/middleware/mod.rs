use crate::{GraphqlError, GraphqlRequest, GraphqlResponse};
use async_trait::async_trait;
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

/// Middleware de autorização para GraphQL.
pub struct AuthMiddleware;

impl AuthMiddleware {
    /// Cria um novo middleware de autorização.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl<State> GraphqlMiddleware<State> for AuthMiddleware
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
        response: &GraphqlResponse,
    ) -> Result<GraphqlResponse, GraphqlError> {
        Ok(response.clone())
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
