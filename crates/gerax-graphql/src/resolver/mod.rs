use crate::GraphqlError;
use async_trait::async_trait;
use serde_json::Value;

/// Trait responsável por resolver campos GraphQL.
///
/// Cada resolver recebe o contexto da requisição e retorna
/// o valor do campo ou um erro.
#[async_trait]
pub trait Resolver<State>: Send + Sync + 'static {
    /// Resolve um campo GraphQL.
    async fn resolve(&self, state: &State, args: Option<&Value>) -> Result<Value, GraphqlError>;
}

/// Resolver para campos de query.
pub struct QueryResolver<State> {
    _marker: std::marker::PhantomData<State>,
}

impl<State> QueryResolver<State> {
    /// Cria um novo resolver de query.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<State> Default for QueryResolver<State> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<State> Resolver<State> for QueryResolver<State>
where
    State: Send + Sync + 'static,
{
    async fn resolve(&self, _state: &State, _args: Option<&Value>) -> Result<Value, GraphqlError> {
        Ok(Value::Null)
    }
}

/// Resolver para campos de mutation.
pub struct MutationResolver<State> {
    _marker: std::marker::PhantomData<State>,
}

impl<State> MutationResolver<State> {
    /// Cria um novo resolver de mutation.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<State> Default for MutationResolver<State> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<State> Resolver<State> for MutationResolver<State>
where
    State: Send + Sync + 'static,
{
    async fn resolve(&self, _state: &State, _args: Option<&Value>) -> Result<Value, GraphqlError> {
        Ok(Value::Null)
    }
}

/// Resolver para campos de subscription.
pub struct SubscriptionResolver<State> {
    _marker: std::marker::PhantomData<State>,
}

impl<State> SubscriptionResolver<State> {
    /// Cria um novo resolver de subscription.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<State> Default for SubscriptionResolver<State> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<State> Resolver<State> for SubscriptionResolver<State>
where
    State: Send + Sync + 'static,
{
    async fn resolve(&self, _state: &State, _args: Option<&Value>) -> Result<Value, GraphqlError> {
        Ok(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{MutationResolver, QueryResolver, Resolver, SubscriptionResolver};

    #[tokio::test]
    async fn default_resolvers_return_null() {
        let query = QueryResolver::<()>::new().resolve(&(), None).await;
        let mutation = MutationResolver::<()>::new().resolve(&(), None).await;
        let subscription = SubscriptionResolver::<()>::new().resolve(&(), None).await;

        assert_eq!(query, Ok(Value::Null));
        assert_eq!(mutation, Ok(Value::Null));
        assert_eq!(subscription, Ok(Value::Null));
    }
}
