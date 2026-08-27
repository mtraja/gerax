use std::sync::Arc;

use gerax_http::{HttpServerBuilder, Middleware, Router, ServerConfig, ServerResult};

use crate::ActixHttpServer;

pub struct ActixHttpServerBuilder<S> {
    state: S,
    router: Option<Arc<Router<S>>>,
    config: ServerConfig,
    middlewares: Vec<Arc<dyn Middleware<S>>>,
    cors_config: Option<gerax_http::CorsConfig>,
}

impl<S> ActixHttpServerBuilder<S>
where
    S: Send + Sync + 'static,
{
    pub fn new(state: S) -> Self {
        Self {
            state,
            router: None,
            config: ServerConfig::default(),
            middlewares: Vec::new(),
            cors_config: None,
        }
    }

    pub fn from_config(state: S, config: ServerConfig) -> Self {
        Self {
            state,
            router: None,
            config,
            middlewares: Vec::new(),
            cors_config: None,
        }
    }

    pub fn config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn route(mut self, router: Router<S>) -> Self {
        self.router = Some(Arc::new(router));
        self
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<S>,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub fn cors(mut self, config: gerax_http::CorsConfig) -> Self {
        self.cors_config = Some(config);
        self
    }
}

impl<S> HttpServerBuilder<S> for ActixHttpServerBuilder<S>
where
    S: Send + Sync + 'static,
{
    type Server = ActixHttpServer<S>;

    fn new(state: S) -> Self {
        Self::new(state)
    }

    fn route(self, router: Router<S>) -> Self {
        Self::route(self, router)
    }

    fn middleware<M>(self, middleware: M) -> Self
    where
        M: Middleware<S>,
    {
        Self::middleware(self, middleware)
    }

    fn config(self, config: ServerConfig) -> Self {
        Self::config(self, config)
    }

    fn build(self) -> ServerResult<Self::Server> {
        Ok(ActixHttpServer {
            state: Arc::new(self.state),
            router: self.router,
            host: self.config.host,
            port: self.config.port,
            middlewares: self.middlewares,
            cors_config: self.cors_config,
        })
    }
}
