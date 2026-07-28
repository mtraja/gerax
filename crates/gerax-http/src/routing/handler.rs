use async_trait::async_trait;

use super::{Context, Response};

#[async_trait]
pub trait Handler<State>: Send + Sync + 'static {
    async fn call(&self, context: Context<State>) -> Response;
}

#[async_trait]
impl<State, F, Fut> Handler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(Context<State>) -> Fut,
    Fut: std::future::Future<Output = Response> + Send,
{
    async fn call(&self, context: Context<State>) -> Response {
        (self)(context).await
    }
}
