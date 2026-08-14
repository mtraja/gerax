use crate::GraphqlError;
use async_graphql::{Schema as AsyncSchema, Variables};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[doc(hidden)]
pub mod private {
    use async_graphql::{ObjectType, SubscriptionType};

    /// Contrato interno que liga uma raiz de query ao executor GraphQL.
    pub trait QueryRoot: ObjectType + Send + Sync + 'static {}

    impl<T> QueryRoot for T where T: ObjectType + Send + Sync + 'static {}

    /// Contrato interno que liga uma raiz de mutation ao executor GraphQL.
    pub trait MutationRoot: ObjectType + Send + Sync + 'static {}

    impl<T> MutationRoot for T where T: ObjectType + Send + Sync + 'static {}

    /// Contrato interno que liga uma raiz de subscription ao executor GraphQL.
    pub trait SubscriptionRoot: SubscriptionType + Send + Sync + 'static {}

    impl<T> SubscriptionRoot for T where T: SubscriptionType + Send + Sync + 'static {}
}

/// Tipo raiz de queries aceito por um [`Schema`].
///
/// Tipos compatíveis com o motor GraphQL interno recebem esta implementação
/// automaticamente; a API pública não exige seus traits diretamente.
pub trait GraphqlQueryRoot: private::QueryRoot {}

impl<T> GraphqlQueryRoot for T where T: private::QueryRoot {}

/// Tipo raiz de mutations aceito por um [`Schema`].
///
/// Tipos compatíveis com o motor GraphQL interno recebem esta implementação
/// automaticamente; a API pública não exige seus traits diretamente.
pub trait GraphqlMutationRoot: private::MutationRoot {}

impl<T> GraphqlMutationRoot for T where T: private::MutationRoot {}

/// Tipo raiz de subscriptions aceito por um [`Schema`].
///
/// Tipos compatíveis com o motor GraphQL interno recebem esta implementação
/// automaticamente; a API pública não exige seus traits diretamente.
pub trait GraphqlSubscriptionRoot: private::SubscriptionRoot {}

impl<T> GraphqlSubscriptionRoot for T where T: private::SubscriptionRoot {}

/// Resposta GraphQL consolidada para retorno ao cliente.
///
/// Agrupa dados, erros e extensões em uma estrutura compatível com o
/// protocolo GraphQL sobre HTTP.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GraphqlExecutedResponse {
    data: Option<Value>,
    errors: Vec<GraphqlError>,
    extensions: Option<Map<String, Value>>,
}

impl GraphqlExecutedResponse {
    /// Cria uma resposta vazia.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define os dados da resposta.
    pub fn with_data(mut self, data: Option<Value>) -> Self {
        self.data = data;
        self
    }

    /// Acrescenta erros à resposta.
    pub fn with_errors(mut self, errors: Vec<GraphqlError>) -> Self {
        self.errors = errors;
        self
    }

    /// Define as extensões da resposta.
    pub fn with_extensions(mut self, extensions: Option<Map<String, Value>>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Retorna `true` quando a resposta contém erros.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Converte a resposta para [`crate::response::GraphqlResponse`].
    pub fn into_response(self) -> crate::response::GraphqlResponse {
        crate::response::GraphqlResponse {
            data: self.data,
            errors: if self.errors.is_empty() {
                None
            } else {
                Some(self.errors)
            },
            extensions: self.extensions,
        }
    }
}

/// Schema GraphQL encapsulado.
///
/// Wraps a schema interna sem expor tipos da async-graphql na API pública.
pub struct Schema<Q, M, S> {
    inner: AsyncSchema<Q, M, S>,
}

impl<Q: 'static, M: 'static, S: 'static> Schema<Q, M, S> {
    /// Cria um novo builder de schema.
    pub fn builder() -> SchemaBuilder<Q, M, S> {
        SchemaBuilder::new()
    }

    /// Executa uma query GraphQL e retorna a resposta consolidada.
    pub async fn execute(
        &self,
        query: &str,
        variables: Option<Map<String, Value>>,
    ) -> Result<GraphqlExecutedResponse, GraphqlError>
    where
        Q: GraphqlQueryRoot,
        M: GraphqlMutationRoot,
        S: GraphqlSubscriptionRoot,
    {
        self.execute_with_state(query, variables, std::sync::Arc::new(()))
            .await
    }

    /// Executa uma query GraphQL com acesso ao estado da aplicação.
    pub async fn execute_with_state<State: Send + Sync + 'static>(
        &self,
        query: &str,
        variables: Option<Map<String, Value>>,
        state: std::sync::Arc<State>,
    ) -> Result<GraphqlExecutedResponse, GraphqlError>
    where
        Q: GraphqlQueryRoot,
        M: GraphqlMutationRoot,
        S: GraphqlSubscriptionRoot,
    {
        let vars = variables.map(|m| Variables::from_json(Value::Object(m)));
        let request = async_graphql::Request::new(query)
            .data(state)
            .variables(vars.unwrap_or_default());
        let response = self.inner.execute(request).await;

        let errors = response
            .errors
            .into_iter()
            .map(|error| GraphqlError::Execution(error.to_string()))
            .collect();

        let raw = serde_json::to_value(response.data).ok();
        let data = match raw {
            Some(serde_json::Value::Object(map)) => Some(serde_json::Value::Object(map)),
            Some(serde_json::Value::Null) => None,
            _ => raw,
        };

        let executed = GraphqlExecutedResponse::new()
            .with_data(data)
            .with_errors(errors);

        Ok(executed)
    }
}

/// Builder para construção de schema GraphQL.
///
/// Permite definir os tipos raiz de query, mutation e subscription
/// antes de construir o schema final.
pub struct SchemaBuilder<Q, M, S> {
    query: Option<Q>,
    mutation: Option<M>,
    subscription: Option<S>,
}

impl<Q, M, S> SchemaBuilder<Q, M, S> {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self {
            query: None,
            mutation: None,
            subscription: None,
        }
    }

    /// Define o tipo raiz de query.
    pub fn query(mut self, query: Q) -> Self {
        self.query = Some(query);
        self
    }

    /// Define o tipo raiz de mutation.
    pub fn mutation(mut self, mutation: M) -> Self {
        self.mutation = Some(mutation);
        self
    }

    /// Define o tipo raiz de subscription.
    pub fn subscription(mut self, subscription: S) -> Self {
        self.subscription = Some(subscription);
        self
    }

    /// Constrói o schema GraphQL.
    ///
    /// A query é obrigatória. Quando mutation ou subscription forem omitidos,
    /// o builder utilizará implementações vazias fornecidas pelo `async-graphql`.
    pub fn finish(self) -> Result<Schema<Q, M, S>, GraphqlError>
    where
        Q: GraphqlQueryRoot,
        M: GraphqlMutationRoot + Default,
        S: GraphqlSubscriptionRoot + Default,
    {
        let query = self
            .query
            .ok_or_else(|| GraphqlError::Validation("query root type is required".to_string()))?;

        let mutation = self.mutation.unwrap_or_default();
        //let mutation = async_graphql::EmptyMutation;
        let subscription = self.subscription.unwrap_or_default();

        let schema = AsyncSchema::build(query, mutation, subscription).finish();

        Ok(Schema { inner: schema })
    }
}

impl<Q, M, S> Default for SchemaBuilder<Q, M, S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use async_graphql::{EmptyMutation, EmptySubscription, Object};
    use serde_json::json;

    use super::{Schema, SchemaBuilder};
    use crate::GraphqlError;

    struct QueryRoot;

    #[Object]
    impl QueryRoot {
        async fn answer(&self) -> i32 {
            42
        }
    }

    #[tokio::test]
    async fn schema_builder_executes_valid_queries() {
        let schema = Schema::builder()
            .query(QueryRoot)
            .mutation(EmptyMutation)
            .subscription(EmptySubscription)
            .finish();

        assert!(schema.is_ok());
        if let Ok(schema) = schema {
            let executed = schema.execute("{ answer }", None).await;

            assert!(executed.is_ok());
            if let Ok(executed) = executed {
                assert_eq!(executed.into_response().data, Some(json!({ "answer": 42 })));
            }
        }
    }

    #[test]
    fn schema_builder_reports_missing_query() {
        let result = SchemaBuilder::<QueryRoot, EmptyMutation, EmptySubscription>::new().finish();

        assert!(matches!(result, Err(GraphqlError::Validation(_))));
    }

    #[tokio::test]
    async fn schema_execute_collects_all_errors() {
        let schema = Schema::builder()
            .query(QueryRoot)
            .mutation(EmptyMutation)
            .subscription(EmptySubscription)
            .finish()
            .expect("schema should be valid");

        let executed = schema
            .execute("query { unknownField }", None)
            .await
            .expect("query should execute");

        assert!(executed.has_errors());
        assert_eq!(executed.into_response().errors.unwrap().len(), 1);
    }
}
