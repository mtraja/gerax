use crate::GraphqlError;
use async_graphql::{ObjectType, Schema as AsyncSchema, SubscriptionType, Variables};
use serde_json::{Map, Value};

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
        Q: ObjectType + Send + Sync,
        M: ObjectType + Send + Sync,
        S: SubscriptionType + Send + Sync,
    {
        let vars = variables.map(|m| Variables::from_json(Value::Object(m)));
        let request = async_graphql::Request::new(query).variables(vars.unwrap_or_default());
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
        Q: ObjectType + Send + Sync + 'static,
        M: ObjectType + Send + Sync + 'static,
        S: SubscriptionType + Send + Sync + 'static,
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
