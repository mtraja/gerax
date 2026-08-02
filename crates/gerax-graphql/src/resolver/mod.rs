use async_trait::async_trait;
use serde_json::Value;
use crate::GraphqlError;

/// Trait responsável por resolver campos GraphQL.
///
/// Cada resolver recebe o contexto da requisição e retorna
/// o valor do campo ou um erro.
#[async_trait]
pub trait Resolver<State>: Send + Sync + 'static {
    /// Resolve um campo GraphQL.
    async fn resolve(
        &self,
        state: &State,
        args: Option<&Value>,
    ) -> Result<Value, GraphqlError>;
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

#[async_trait]
impl<State> Resolver<State> for QueryResolver<State>
where
    State: Send + Sync + 'static,
{
    async fn resolve(
        &self,
        _state: &State,
        _args: Option<&Value>,
    ) -> Result<Value, GraphqlError> {
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

#[async_trait]
impl<State> Resolver<State> for MutationResolver<State>
where
    State: Send + Sync + 'static,
{
    async fn resolve(
        &self,
        _state: &State,
        _args: Option<&Value>,
    ) -> Result<Value, GraphqlError> {
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

#[async_trait]
impl<State> Resolver<State> for SubscriptionResolver<State>
where
    State: Send + Sync + 'static,
{
    async fn resolve(
        &self,
        _state: &State,
        _args: Option<&Value>,
    ) -> Result<Value, GraphqlError> {
        Ok(Value::Null)
    }
}