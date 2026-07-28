use super::{Handler, HttpMethod, Route, Scope};
use crate::routing::{Context, Response};
use crate::{HttpServerError, ServerResult};
use crate::Middleware;
use matchit::Router as MatchitRouter;
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
    router: MatchitRouter<Route<State>>,
    routes: Vec<Route<State>>,
    scopes: Vec<Scope<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State> Router<State> {
    pub fn new() -> Self {
        Self {
            router: MatchitRouter::new(),
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
        let path = path.into();
        let route = Route::new(method, path.clone(), handler);
        self.router.insert(&path, route.clone()).expect("duplicate route");
        self.routes.push(route);
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
        M: Middleware<State>,
    {
        self.middlewares.push(Arc::new(middleware));

        self
    }

    // ---------------------------------------------------------
    // Merge
    // ---------------------------------------------------------

    pub fn merge(mut self, other: Router<State>) -> Self {
        let start = self.routes.len();
        self.routes.extend(other.routes);
        for route in &self.routes[start..] {
            self.router.insert(route.path(), route.clone()).expect("duplicate route");
        }
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

        let match_result = self.router.at(&path);

        match match_result {
            Ok(m) => {
                let route = m.value;
                if route.method() != method {
                    return Err(HttpServerError::HandlerError("Method not allowed".to_string()));
                }

                for (key, value) in m.params.iter() {
                    ctx.params_mut().insert(key.to_string(), value.to_string());
                }

                route.execute(ctx).await
            }
            Err(_) => {
                let mut scopes_to_try: Vec<_> = self.scopes.iter().collect();
                while let Some(scope) = scopes_to_try.pop() {
                    for sub_scope in scope.scopes().iter() {
                        scopes_to_try.push(sub_scope);
                    }

                    match scope.handle(ctx.clone()).await {
                        Ok(response) => return Ok(response),
                        Err(HttpServerError::HandlerError(_)) => {}
                        Err(e) => return Err(e),
                    }
                }

                Err(HttpServerError::HandlerError("Route not found".to_string()))
            }
        }
    }
}
