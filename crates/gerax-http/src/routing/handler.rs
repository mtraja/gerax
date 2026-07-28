use async_trait::async_trait;

use super::{Context, Response};
use crate::ServerResult;

#[async_trait]
pub trait Handler<State>: Send + Sync + 'static {
    async fn call(&self, context: Context<State>) -> ServerResult<Response>;
}

#[async_trait]
impl<State, F, Fut> Handler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(Context<State>) -> Fut,
    Fut: std::future::Future<Output = ServerResult<Response>> + Send,
{
    async fn call(&self, context: Context<State>) -> ServerResult<Response> {
        (self)(context).await
    }
}
