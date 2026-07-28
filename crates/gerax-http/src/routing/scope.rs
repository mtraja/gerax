use matchit::Router as MatchitRouter;
use std::sync::Arc;

use crate::routing::{Context, Response};
use crate::{HttpServerError, ServerResult};
use crate::Middleware;

use super::{Handler, HttpMethod, Route};

struct RouteData<State> {
    method: HttpMethod,
    path_pattern: String,
    full_path: String,
    handler: Arc<dyn Handler<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State> Clone for RouteData<State> {
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            path_pattern: self.path_pattern.clone(),
            full_path: self.full_path.clone(),
            handler: Arc::clone(&self.handler),
            middlewares: self.middlewares.clone(),
        }
    }
}

pub struct Scope<State> {
    prefix: String,
    router: MatchitRouter<RouteData<State>>,
    routes_data: Vec<RouteData<State>>,
    scopes: Vec<Scope<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State> Scope<State> {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            router: MatchitRouter::new(),
            routes_data: Vec::new(),
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
        let path_str = path.into();
        let full_path = format!("{}{}", self.prefix, path_str);
        let data = RouteData {
            method,
            path_pattern: path_str.clone(),
            full_path: full_path.clone(),
            handler: Arc::new(handler),
            middlewares: Vec::new(),
        };
        self.router.insert(&full_path, data.clone()).expect("duplicate route");
        self.routes_data.push(data);
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

    pub fn routes(&self) -> Vec<Route<State>>
    where
        State: Send + Sync + 'static,
    {
        self.routes_data
            .iter()
            .map(|data| {
                let mut route = Route::with_handler(data.method.clone(), data.path_pattern.clone(), Arc::clone(&data.handler));
                route.set_middlewares(data.middlewares.clone());
                route
            })
            .collect()
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

    pub async fn handle(&self, mut ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        let path = ctx.request().path().to_string();
        let method = ctx.request().method().clone();

        let match_result = self.router.at(&path)
            .map_err(|e| HttpServerError::HandlerError(e.to_string()))?;

        let route_data = match_result.value;
        if route_data.method.clone() != method {
            return Err(HttpServerError::HandlerError("Method not allowed".to_string()));
        }

        for (key, value) in match_result.params.iter() {
            ctx.params_mut().insert(key.to_string(), value.to_string());
        }

        let mut route = Route::with_handler(
            route_data.method.clone(),
            route_data.path_pattern.clone(),
            Arc::clone(&route_data.handler),
        );
        route.set_middlewares(route_data.middlewares.clone());
        route.execute(ctx).await
    }
}
