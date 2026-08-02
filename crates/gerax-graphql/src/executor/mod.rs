use async_trait::async_trait;
use crate::{GraphqlRequest, GraphqlResponse, GraphqlError, Schema};

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
    Q: async_graphql::ObjectType + Send + Sync + 'static,
    M: async_graphql::ObjectType + Send + Sync + 'static,
    S: async_graphql::SubscriptionType + Send + Sync + 'static,
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