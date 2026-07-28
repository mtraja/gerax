use std::sync::Arc;

use crate::routing::{Context, Response};
use crate::{HttpServerError, ServerResult};
use crate::Middleware;

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

    // ---------------------------------------------------------
    // Route
    // ---------------------------------------------------------

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

    // ---------------------------------------------------------
    // Nested Scope
    // ---------------------------------------------------------

    pub fn scope(mut self, scope: Scope<State>) -> Self {
        self.scopes.push(scope);
        self
    }

    // ---------------------------------------------------------
    // Middleware
    // ---------------------------------------------------------

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<State>,
    {
        self.middlewares.push(Arc::new(middleware));

        self
    }

    // ---------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------

    pub fn routes(&self) -> &[Route<State>] {
        &self.routes
    }

    pub fn scopes(&self) -> &[Scope<State>] {
        &self.scopes
    }

    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>] {
        &self.middlewares
    }

    // ---------------------------------------------------------
    // Handle
    // ---------------------------------------------------------

    pub async fn handle(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        let path = ctx.request().path();
        let method = ctx.request().method();

        if let Some(route) = self.routes.iter().find(|route| {
            route.method() == *method && route.path() == path
        }) {
            route.execute(ctx).await
        } else {
            Err(HttpServerError::HandlerError(
                "Route not found in scope".to_string(),
            ))
        }
    }
}
