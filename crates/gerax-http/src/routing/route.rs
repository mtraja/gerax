use super::{Handler, HttpMethod};
use crate::middleware::Next;
use crate::routing::{Context, Response};
use crate::ServerResult;
use crate::Middleware;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct Route<State> {
    method: HttpMethod,
    path: String,
    handler: Arc<dyn Handler<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State> Route<State> {
    pub fn new<H>(method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        Self {
            method,
            path: path.into(),
            handler: Arc::new(handler),
            middlewares: Vec::new(),
        }
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn handler(&self) -> &Arc<dyn Handler<State>> {
        &self.handler
    }

    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>] {
        &self.middlewares
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<State>,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub async fn execute(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        let handler = Arc::clone(&self.handler);
        let mut continuation: Box<
            dyn FnOnce(Context<State>) -> Pin<Box<dyn Future<Output = ServerResult<Response>> + Send>>
                + Send
                + Sync,
        > = Box::new(move |ctx| {
            let handler = Arc::clone(&handler);
            Box::pin(async move { handler.call(ctx).await })
        });

        for mw in self.middlewares.iter().rev() {
            let mw = Arc::clone(mw);
            let prev_continuation = continuation;
            continuation = Box::new(move |ctx| {
                let prev = prev_continuation;
                let mw = Arc::clone(&mw);
                Box::pin(async move {
                    let next = Next::new(prev);
                    mw.handle(ctx, next).await
                })
            });
        }

        continuation(ctx).await
    }
}
