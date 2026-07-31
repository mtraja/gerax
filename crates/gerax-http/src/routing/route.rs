use super::{Handler, HttpMethod};
use crate::Middleware;
use crate::ServerResult;
use crate::middleware::{Next, NextFn};
use crate::routing::{Context, PathParams, Response};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Route<State> {
    method: HttpMethod,
    path_pattern: String,
    path: String,
    handler: Arc<dyn Handler<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State> Clone for Route<State> {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            path_pattern: self.path_pattern.clone(),
            path: self.path.clone(),
            handler: Arc::clone(&self.handler),
            middlewares: self.middlewares.clone(),
        }
    }
}

impl<State> Route<State> {
    pub fn new<H>(method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        let path_str = path.into();
        Self {
            method,
            path_pattern: path_str.clone(),
            path: path_str,
            handler: Arc::new(handler),
            middlewares: Vec::new(),
        }
    }

    pub fn method(&self) -> HttpMethod {
        self.method.clone()
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn path_pattern(&self) -> &str {
        &self.path_pattern
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

    pub fn with_handler(
        method: HttpMethod,
        path: impl Into<String>,
        handler: Arc<dyn Handler<State>>,
    ) -> Self
    where
        State: Send + Sync + 'static,
    {
        let path_str = path.into();
        Self {
            method,
            path_pattern: path_str.clone(),
            path: path_str,
            handler,
            middlewares: Vec::new(),
        }
    }

    pub fn set_middlewares(&mut self, middlewares: Vec<Arc<dyn Middleware<State>>>) {
        self.middlewares = middlewares;
    }

    pub(crate) fn with_parts(
        method: HttpMethod,
        path: String,
        handler: Arc<dyn Handler<State>>,
        middlewares: Vec<Arc<dyn Middleware<State>>>,
    ) -> Self {
        Self {
            method,
            path_pattern: path.clone(),
            path,
            handler,
            middlewares,
        }
    }

    pub(crate) fn with_prefix_and_middlewares(
        &self,
        prefix: &str,
        inherited_middlewares: &[Arc<dyn Middleware<State>>],
    ) -> Self {
        let mut middlewares = inherited_middlewares.to_vec();
        middlewares.extend(self.middlewares.clone());
        Self::with_parts(
            self.method.clone(),
            join_paths(prefix, &self.path),
            Arc::clone(&self.handler),
            middlewares,
        )
    }

    pub fn extract_params<V>(&self, matched: &matchit::Match<'_, '_, V>) -> PathParams {
        let mut params = HashMap::new();
        for (key, value) in matched.params.iter() {
            params.insert(key.to_string(), value.to_string());
        }
        PathParams::new(params)
    }

    pub async fn execute(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        let handler = Arc::clone(&self.handler);
        let mut continuation: Box<NextFn<State>> = Box::new(move |ctx| {
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

pub(crate) fn join_paths(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    let path = path.trim_start_matches('/');

    match (prefix, path) {
        ("", "") => "/".to_string(),
        ("", path) => format!("/{path}"),
        (prefix, "") => prefix.to_string(),
        (prefix, path) => format!("{prefix}/{path}"),
    }
}
