use async_trait::async_trait;

use super::{Request, Response};

#[async_trait]
pub trait Handler<State>: Send + Sync + 'static {
    async fn call(&self, request: Request<State>) -> Response;
}

#[async_trait]
impl<State, F, Fut> Handler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(Request<State>) -> Fut,
    Fut: std::future::Future<Output = Response> + Send,
{
    async fn call(&self, request: Request<State>) -> Response {
        (self)(request).await
    }
}
