use std::sync::Arc;

use crate::routing::{Context, Response, join_paths};
use crate::{Middleware, ServerResult};

use super::{Handler, HttpMethod, Route};

pub struct Scope<State> {
    prefix: String,
    routes: Vec<Route<State>>,
    scopes: Vec<Scope<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State> Scope<State> {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            routes: Vec::new(),
            scopes: Vec::new(),
            middlewares: Vec::new(),
        }
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn route<H>(mut self, method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.routes.push(Route::new(method, path, handler));
        self
    }

    pub fn get<H>(self, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.route(HttpMethod::Get, path, handler)
    }
    pub fn post<H>(self, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.route(HttpMethod::Post, path, handler)
    }
    pub fn put<H>(self, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.route(HttpMethod::Put, path, handler)
    }
    pub fn patch<H>(self, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.route(HttpMethod::Patch, path, handler)
    }
    pub fn delete<H>(self, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.route(HttpMethod::Delete, path, handler)
    }
    pub fn head<H>(self, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.route(HttpMethod::Head, path, handler)
    }
    pub fn options<H>(self, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.route(HttpMethod::Options, path, handler)
    }

    pub fn scope(mut self, scope: Scope<State>) -> Self {
        self.scopes.push(scope);
        self
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<State>,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub fn routes(&self) -> &[Route<State>] {
        &self.routes
    }
    pub fn scopes(&self) -> &[Scope<State>] {
        &self.scopes
    }
    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>] {
        &self.middlewares
    }

    pub(crate) fn flatten(
        &self,
        parent_prefix: &str,
        inherited_middlewares: &[Arc<dyn Middleware<State>>],
    ) -> Vec<Route<State>> {
        let prefix = join_paths(parent_prefix, &self.prefix);
        let mut middlewares = inherited_middlewares.to_vec();
        middlewares.extend(self.middlewares.clone());

        let mut routes: Vec<_> = self
            .routes
            .iter()
            .map(|route| route.with_prefix_and_middlewares(&prefix, &middlewares))
            .collect();
        for scope in &self.scopes {
            routes.extend(scope.flatten(&prefix, &middlewares));
        }
        routes
    }

    pub async fn handle(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        let mut router = super::Router::new();
        for route in self.flatten("", &[]) {
            router = router.route(route.method(), route.path().to_string(), move |ctx| {
                let route = route.clone();
                async move { route.execute(ctx).await }
            });
        }
        router.handle(ctx).await
    }
}
