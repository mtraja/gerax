//! Porta de entrada para execução de Queries.

use std::sync::Arc;

use crate::error::CqrsError;
use crate::query::Query;
use crate::registry::HandlerRegistry;

/// Executa [`Query`]s tipadamente.
///
/// O usuário não precisa conhecer `Any`, `TypeId`, `ErasedHandler`
/// ou `HandlerAdapter`; esses são detalhes internos da infraestrutura.
#[derive(Clone)]
pub struct QueryBus {
    registry: Arc<HandlerRegistry>,
}

impl QueryBus {
    pub fn new(registry: Arc<HandlerRegistry>) -> Self {
        Self { registry }
    }

    /// Executa uma `Query` e retorna o resultado tipado.
    pub async fn execute<Q>(&self, query: Q) -> Result<Q::Output, CqrsError>
    where
        Q: Query,
    {
        self.registry.dispatch_query(query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct GetAnswer;

    impl crate::Message for GetAnswer {
        type Output = Option<i32>;
    }

    impl Query for GetAnswer {}

    struct GetAnswerHandler;

    #[async_trait::async_trait]
    impl crate::QueryHandler for GetAnswerHandler {
        type Query = GetAnswer;

        async fn handle(&self, _query: GetAnswer) -> Result<Option<i32>, CqrsError> {
            Ok(Some(42))
        }
    }

    #[tokio::test]
    async fn execute_retorna_output_tipado() {
        let mut registry = HandlerRegistry::new();
        registry.register_query(GetAnswerHandler).unwrap();

        let bus = QueryBus::new(Arc::new(registry));

        let value: Option<i32> = bus.execute(GetAnswer).await.unwrap();
        assert_eq!(value, Some(42));
    }

    #[tokio::test]
    async fn execute_sem_handler_retorna_not_found() {
        let bus = QueryBus::new(Arc::new(HandlerRegistry::new()));

        let err = bus.execute(GetAnswer).await.unwrap_err();
        assert!(matches!(err, CqrsError::HandlerNotFound { .. }));
    }
}
