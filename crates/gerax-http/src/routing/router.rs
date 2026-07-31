use std::sync::Arc;

use matchit::Router as MatchitRouter;

use super::{Context, Handler, HttpMethod, Response, Route, Scope};
use crate::{HttpServerError, Middleware, ServerResult};

pub struct Router<State> {
    router: MatchitRouter<Vec<Route<State>>>,
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

    pub fn route<H>(mut self, method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        self.routes.push(Route::new(method, path, handler));
        self.rebuild_matcher();
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
        self.rebuild_matcher();
        self
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<State>,
    {
        self.middlewares.push(Arc::new(middleware));
        self.rebuild_matcher();
        self
    }

    pub fn merge(mut self, other: Router<State>) -> Self {
        self.routes.extend(other.routes);
        self.scopes.extend(other.scopes);
        self.middlewares.extend(other.middlewares);
        self.rebuild_matcher();
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

    fn effective_routes(&self) -> Vec<Route<State>> {
        let mut routes: Vec<_> = self
            .routes
            .iter()
            .map(|route| route.with_prefix_and_middlewares("", &self.middlewares))
            .collect();
        for scope in &self.scopes {
            routes.extend(scope.flatten("", &self.middlewares));
        }
        routes
    }

    fn rebuild_matcher(&mut self) {
        self.router = build_matcher(self.effective_routes());
    }

    pub async fn handle(&self, mut ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        let path = ctx.request().path().to_string();
        let method = ctx.request().method().clone();
        let matched = self
            .router
            .at(&path)
            .map_err(|_| HttpServerError::HandlerError("Route not found".to_string()))?;
        let route = matched
            .value
            .iter()
            .find(|route| route.method() == method)
            .ok_or_else(|| HttpServerError::HandlerError("Method not allowed".to_string()))?;
        for (key, value) in matched.params.iter() {
            ctx.params_mut().insert(key.to_string(), value.to_string());
        }
        route.execute(ctx).await
    }
}

impl<State> Default for Router<State> {
    fn default() -> Self {
        Self::new()
    }
}

fn build_matcher<State>(routes: Vec<Route<State>>) -> MatchitRouter<Vec<Route<State>>> {
    let mut groups: Vec<(String, Vec<Route<State>>)> = Vec::new();
    for route in routes {
        let path = route.path().to_string();
        if let Some((_, routes)) = groups
            .iter_mut()
            .find(|(registered, _)| registered == &path)
        {
            assert!(
                !routes
                    .iter()
                    .any(|registered| registered.method() == route.method()),
                "duplicate route for {:?} {}",
                route.method(),
                path
            );
            routes.push(route);
        } else {
            groups.push((path, vec![route]));
        }
    }
    let mut matcher = MatchitRouter::new();
    for (path, routes) in groups {
        matcher
            .insert(&path, routes)
            .expect("invalid route pattern");
    }
    matcher
}
