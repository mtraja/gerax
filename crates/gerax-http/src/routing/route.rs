use super::{Handler, HttpMethod};
use crate::Middleware;
use std::sync::Arc;
//use crate ::middleware::Middleware;

pub struct Route<State> {
    method: HttpMethod,
    path: String,
    handler: Arc<dyn Handler<State>>,
    middlewares: Vec<Arc<dyn Middleware>>,
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

    pub fn middlewares(&self) -> &[Arc<dyn Middleware>] {
        &self.middlewares
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }
}
