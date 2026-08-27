use crate::{Executor, GraphqlError, GraphqlRequest, GraphqlResponse};
use async_trait::async_trait;
use gerax_http::routing::Response;
use gerax_http::routing::context::Context as HttpContext;
use gerax_http::routing::handler::Handler;
use gerax_http::{HttpServerError, ServerResult};

/// Handler GraphQL que implementa `Handler<State>` do `gerax-http`.
///
/// Assim GraphQL será apenas mais uma rota no router.
///
/// ## Exemplo
///
/// ```ignore
/// use gerax_http::Router;
/// use gerax_graphql::{GraphqlHandler, Executor, Schema};
///
/// let schema = Schema::builder()
///     .query(QueryRoot)
///     .finish()?;
///
/// let executor = GraphqlHandler::new(schema);
/// let router = Router::new()
///     .post("/graphql", executor);
/// ```
pub struct GraphqlHandler<State> {
    executor: Box<dyn Executor<State>>,
}

impl<State> GraphqlHandler<State> {
    /// Cria um novo handler GraphQL a partir de um executor.
    pub fn new<E: Executor<State> + 'static>(executor: E) -> Self {
        Self {
            executor: Box::new(executor),
        }
    }

    /// Extrai a requisição GraphQL do contexto HTTP.
    fn extract_request(
        &self,
        context: &HttpContext<State>,
    ) -> Result<GraphqlRequest, GraphqlError> {
        let body = context.request().body();
        let request: GraphqlRequest =
            serde_json::from_slice(body).map_err(|e| GraphqlError::Validation(e.to_string()))?;
        Ok(request)
    }

    /// Converte a resposta GraphQL em uma resposta HTTP.
    fn to_response(result: Result<GraphqlResponse, GraphqlError>) -> ServerResult<Response> {
        match result {
            Ok(response) => {
                let body = serde_json::to_vec(&response)
                    .map_err(|e| HttpServerError::RuntimeError(e.to_string()))?;
                Ok(Response::ok(body))
            }
            Err(e) => {
                let error_response = GraphqlResponse {
                    data: None,
                    errors: Some(vec![e]),
                    extensions: None,
                };
                let body = serde_json::to_vec(&error_response)
                    .map_err(|e| HttpServerError::RuntimeError(e.to_string()))?;
                Ok(Response {
                    status: 400,
                    body,
                    ..Default::default()
                })
            }
        }
    }
}

#[async_trait]
impl<State> Handler<State> for GraphqlHandler<State>
where
    State: Send + Sync + 'static,
{
    async fn call(&self, context: HttpContext<State>) -> ServerResult<Response> {
        let request = match self.extract_request(&context) {
            Ok(req) => req,
            Err(e) => return Self::to_response(Err(e)),
        };
        let state = context.state();
        let result = self.executor.execute(request, &*state).await;
        Self::to_response(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;

    use super::GraphqlHandler;
    use crate::{Executor, GraphqlError, GraphqlRequest, GraphqlResponse};
    use gerax_http::routing::{Context, Handler, HttpMethod, Request};

    struct SuccessExecutor;

    #[async_trait]
    impl Executor<()> for SuccessExecutor {
        async fn execute(
            &self,
            request: GraphqlRequest,
            _state: &(),
        ) -> Result<GraphqlResponse, GraphqlError> {
            Ok(GraphqlResponse {
                data: Some(json!({ "query": request.query })),
                ..GraphqlResponse::default()
            })
        }
    }

    struct FailingExecutor;

    #[async_trait]
    impl Executor<()> for FailingExecutor {
        async fn execute(
            &self,
            _request: GraphqlRequest,
            _state: &(),
        ) -> Result<GraphqlResponse, GraphqlError> {
            Err(GraphqlError::Execution("resolver failed".to_string()))
        }
    }

    fn context(body: &[u8]) -> Context<()> {
        Context::new(
            Arc::new(()),
            Request::new(HttpMethod::Post, "/graphql".to_string(), body.to_vec()),
        )
    }

    #[tokio::test]
    async fn handler_serializes_executor_responses() {
        let handler = GraphqlHandler::new(SuccessExecutor);
        let response = handler
            .call(context(br#"{"query":"{ viewer { id } }"}"#))
            .await;

        assert!(response.is_ok());
        if let Ok(response) = response {
            assert_eq!(response.status, 200);
            assert!(String::from_utf8_lossy(&response.body).contains("viewer"));
        }
    }

    #[tokio::test]
    async fn handler_returns_bad_request_for_executor_errors() {
        let handler = GraphqlHandler::new(FailingExecutor);
        let response = handler
            .call(context(br#"{"query":"{ viewer { id } }"}"#))
            .await;

        assert!(response.is_ok());
        if let Ok(response) = response {
            assert_eq!(response.status, 400);
            assert!(String::from_utf8_lossy(&response.body).contains("resolver failed"));
        }
    }
}
