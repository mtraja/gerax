use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("Server start error: {0}")]
    StartError(String),
    #[error("Route not found")]
    NotFound,
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl HttpResponse {
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
            headers: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct Route {
    pub path: String,
    pub method: HttpMethod,
    pub handler: Arc<dyn Fn() -> Result<HttpResponse, HttpError> + Send + Sync>,
}

pub struct RouteConfig {
    routes: Vec<Route>,
}

impl RouteConfig {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn route(&mut self, method: HttpMethod, path: &str, handler: impl Fn() -> Result<HttpResponse, HttpError> + Send + Sync + 'static) -> &mut Self {
        let handler = Arc::new(handler);
        self.routes.push(Route {
            path: path.to_string(),
            method,
            handler,
        });
        self
    }

    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}

impl Default for RouteConfig {
    fn default() -> Self {
        Self::new()
    }
}

pub trait RouteConfigurator<S: Send + Sync + Clone + 'static> {
    fn configure(&self, state: S) -> Vec<Route>;
}

pub trait HttpServerTrait<S: Send + Sync + Clone + 'static> {
    async fn start(&self, state: S) -> Result<(), HttpError>;
}

pub struct HttpServer<S: Send + Sync + Clone + 'static> {
    _marker: std::marker::PhantomData<S>,
}

impl<S: Send + Sync + Clone + 'static> HttpServerTrait<S> for HttpServer<S> {
    async fn start(&self, _state: S) -> Result<(), HttpError> {
        Err(HttpError::Internal("Not implemented".to_string()))
    }
}