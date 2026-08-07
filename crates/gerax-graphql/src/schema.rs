use crate::GraphqlError;
use async_graphql::{Schema as AsyncSchema, Variables};
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

    /// Executa uma query GraphQL.
    pub async fn execute(
        &self,
        query: &str,
        variables: Option<Map<String, Value>>,
    ) -> Result<Value, GraphqlError>
    where
        Q: GraphqlQueryRoot,
        M: GraphqlMutationRoot,
        S: GraphqlSubscriptionRoot,
    {
        self.execute_with_state(query, variables, std::sync::Arc::new(())).await
    }

    /// Executa uma query GraphQL com acesso ao estado da aplicação.
    pub async fn execute_with_state<State: Send + Sync + 'static>(
        &self,
        query: &str,
        variables: Option<Map<String, Value>>,
        state: std::sync::Arc<State>,
    ) -> Result<Value, GraphqlError>
    where
        Q: GraphqlQueryRoot,
        M: GraphqlMutationRoot,
        S: GraphqlSubscriptionRoot,
    {
        let vars = variables.map(|m| Variables::from_json(Value::Object(m)));
        let request =
            async_graphql::Request::new(query)
                .data(state)
                .variables(vars.unwrap_or_default());
        let response = self.inner.execute(request).await;

        if let Some(errors) = response.errors.first() {
            return Err(GraphqlError::Execution(errors.to_string()));
        }

        serde_json::to_value(response.data).map_err(|e| GraphqlError::Execution(e.to_string()))
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
    pub fn finish(self) -> Result<Schema<Q, M, S>, GraphqlError>
    where
        Q: GraphqlQueryRoot,
        M: GraphqlMutationRoot,
        S: GraphqlSubscriptionRoot,
    {
        let query = self
            .query
            .ok_or_else(|| GraphqlError::Validation("query root type is required".to_string()))?;

        let mutation = self.mutation.ok_or_else(|| {
            GraphqlError::Validation("mutation root type is required".to_string())
        })?;

        let subscription = self.subscription.ok_or_else(|| {
            GraphqlError::Validation("subscription root type is required".to_string())
        })?;

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
            assert_eq!(
                schema.execute("{ answer }", None).await,
                Ok(json!({ "answer": 42 }))
            );
        }
    }

    #[test]
    fn schema_builder_reports_missing_roots() {
        let result = SchemaBuilder::<QueryRoot, EmptyMutation, EmptySubscription>::new().finish();

        assert!(matches!(result, Err(GraphqlError::Validation(_))));
    }
}
