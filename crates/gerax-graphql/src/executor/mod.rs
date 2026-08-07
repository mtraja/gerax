use crate::{
    GraphqlError, GraphqlMutationRoot, GraphqlQueryRoot, GraphqlRequest, GraphqlResponse,
    GraphqlSubscriptionRoot, Schema,
};
use async_trait::async_trait;

/// Trait responsável por executar operações GraphQL.
///
/// Nunca depende diretamente de HTTP.
/// A implementação deve receber uma requisição GraphQL e o estado da aplicação,
/// retornando a resposta ou um erro.
#[async_trait]
pub trait Executor<State>: Send + Sync + 'static {
    /// Executa uma operação GraphQL.
    async fn execute(
        &self,
        request: GraphqlRequest,
        state: &State,
    ) -> Result<GraphqlResponse, GraphqlError>;
}

/// Executor padrão que utiliza um `Schema` para executar queries.
pub struct DefaultExecutor<State, Q, M, S> {
    schema: Schema<Q, M, S>,
    _marker: std::marker::PhantomData<State>,
}

impl<State, Q, M, S> DefaultExecutor<State, Q, M, S> {
    /// Cria um novo executor a partir de um schema.
    pub fn new(schema: Schema<Q, M, S>) -> Self {
        Self {
            schema,
            _marker: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<State, Q, M, S> Executor<State> for DefaultExecutor<State, Q, M, S>
where
    State: Send + Sync + 'static,
    Q: GraphqlQueryRoot,
    M: GraphqlMutationRoot,
    S: GraphqlSubscriptionRoot,
{
    async fn execute(
        &self,
        request: GraphqlRequest,
        _state: &State,
    ) -> Result<GraphqlResponse, GraphqlError> {
        let data = self
            .schema
            .execute(&request.query, request.variables)
            .await?;

        Ok(GraphqlResponse {
            data: Some(data),
            errors: None,
            extensions: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use async_graphql::{EmptyMutation, EmptySubscription, Object};
    use serde_json::json;

    use super::{DefaultExecutor, Executor};
    use crate::{GraphqlRequest, Schema};

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        async fn greeting(&self) -> &str {
            "hello"
        }
    }

    #[tokio::test]
    async fn default_executor_converts_schema_results_to_responses() {
        let schema = Schema::builder()
            .query(QueryRoot)
            .mutation(EmptyMutation)
            .subscription(EmptySubscription)
            .finish();

        assert!(schema.is_ok());
        if let Ok(schema) = schema {
            let executor = DefaultExecutor::<(), _, _, _>::new(schema);
            let response = executor
                .execute(
                    GraphqlRequest {
                        query: "{ greeting }".to_string(),
                        ..GraphqlRequest::default()
                    },
                    &(),
                )
                .await;

            assert_eq!(
                response.map(|value| value.data),
                Ok(Some(json!({ "greeting": "hello" })))
            );
        }
    }
}
