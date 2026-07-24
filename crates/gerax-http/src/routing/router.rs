use super::{Handler, HttpMethod, Route, Scope};
use crate::Middleware;
use std::sync::Arc;


/// Router::new()
///     .middleware(Logger)
///     .scope(
///         Scope::new("/api")
///             .middleware(Auth)
///             .get("/users", list_users)
///             .scope(
///                 Scope::new("/admin")
///                     .middleware(AdminAuth)
///                     .get("/dashboard", dashboard)
///             )
///      );
///

pub struct Router<State> {
    routes: Vec<Route<State>>,
    scopes: Vec<Scope<State>>,
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl<State> Router<State> {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            scopes: Vec::new(),
            middlewares: Vec::new(),
        }
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
    // Scope
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
        M: Middleware,
    {
        self.middlewares.push(Arc::new(middleware));

        self
    }

    // ---------------------------------------------------------
    // Merge
    // ---------------------------------------------------------

    pub fn merge(mut self, other: Router<State>) -> Self {
        self.routes.extend(other.routes);
        self.scopes.extend(other.scopes);
        self.middlewares.extend(other.middlewares);

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

    pub fn middlewares(&self) -> &[Arc<dyn Middleware>] {
        &self.middlewares
    }
}
