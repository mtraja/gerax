use crate::{
    GraphqlContext, GraphqlError, GraphqlMutationRoot, GraphqlQueryRoot, GraphqlRequest,
    GraphqlResponse, GraphqlSubscriptionRoot, Schema,
};
use async_trait::async_trait;
use gerax_auth::Claims;

/// Trait responsável por executar operações GraphQL.
///
/// Recebe o contexto HTTP do `gerax-http` para que tenha acesso ao estado da
/// aplicação (`Context::state()`) e às extensões da requisição — onde o
/// `gerax_auth::AuthMiddleware` injeta as [`Claims`] do usuário autenticado.
#[async_trait]
pub trait Executor<State>: Send + Sync + 'static {
    /// Executa uma operação GraphQL.
    async fn execute(
        &self,
        request: GraphqlRequest,
        context: &GraphqlContext<State>,
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
    State: Send + Sync + Clone + 'static,
    Q: GraphqlQueryRoot,
    M: GraphqlMutationRoot,
    S: GraphqlSubscriptionRoot,
{
    async fn execute(
        &self,
        request: GraphqlRequest,
        context: &GraphqlContext<State>,
    ) -> Result<GraphqlResponse, GraphqlError> {
        let state = context.state();
        let claims = context
            .extensions()
            .get::<Claims>()
            .map(|claims| claims.as_ref().clone());

        let executed = self
            .schema
            .execute_with_data(&request.query, request.variables, Some(state), claims)
            .await?;

        Ok(executed.into_response())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_graphql::{EmptyMutation, EmptySubscription, Object};
    use gerax_http::routing::{Context, HttpMethod, Request};
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

    fn context() -> Context<()> {
        Context::new(
            Arc::new(()),
            Request::new(HttpMethod::Post, "/graphql".to_string(), Vec::new()),
        )
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
                    &context(),
                )
                .await;

            assert_eq!(
                response.map(|value| value.data),
                Ok(Some(json!({ "greeting": "hello" })))
            );
        }
    }

    #[tokio::test]
    async fn default_executor_forwards_state_and_claims_to_resolvers() {
        use gerax_auth::Claims;

        #[derive(Clone)]
        struct State(i32);

        struct QueryRoot;

        #[Object]
        impl QueryRoot {
            async fn from_state(&self, ctx: &async_graphql::Context<'_>) -> i32 {
                ctx.data::<State>().unwrap().0
            }

            async fn from_claims(&self, ctx: &async_graphql::Context<'_>) -> String {
                ctx.data::<Claims>().unwrap().sub.clone()
            }
        }

        let schema = Schema::builder()
            .query(QueryRoot)
            .mutation(EmptyMutation)
            .subscription(EmptySubscription)
            .finish()
            .expect("schema should be valid");

        let executor = DefaultExecutor::<State, _, _, _>::new(schema);
        let mut context = Context::new(
            Arc::new(State(7)),
            Request::new(HttpMethod::Post, "/graphql".to_string(), Vec::new()),
        );
        context.extensions_mut().insert(Claims {
            sub: "user-1".to_string(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["clinica:read".to_string()],
        });

        let response = executor
            .execute(
                GraphqlRequest {
                    query: "{ fromState fromClaims }".to_string(),
                    ..GraphqlRequest::default()
                },
                &context,
            )
            .await
            .expect("execution should succeed");

        assert_eq!(
            response.data,
            Some(json!({ "fromState": 7, "fromClaims": "user-1" }))
        );
    }
}
