use std::sync::Arc;

use gerax_config::builder::ConfigBuilder;

use crate::{ActixConfig, ActixHttpServer};

use gerax_http::{HttpServerBuilder, Middleware, Router, ServerResult};

pub struct ActixHttpServerBuilder<S> {
    state: S,
    router: Option<Arc<Router<S>>>,
    config: ActixConfig,
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl<S> ActixHttpServerBuilder<S>
where
    S: Send + Sync + 'static,
{
    pub fn new(state: S) -> Self {
        Self {
            state,
            router: None,
            config: ActixConfig::default(),
            middlewares: Vec::new(),
        }
    }

    pub fn from_config(state: S, config: ActixConfig) -> Self {
        Self {
            state,
            router: None,
            config,
            middlewares: Vec::new(),
        }
    }

    pub fn config(mut self, config: ActixConfig) -> Self {
        self.config = config;
        self
    }

    pub fn route(mut self, router: Router<S>) -> Self {
        self.router = Some(Arc::new(router));
        self
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware,
    {
        self.middlewares.push(Arc::new(middleware));
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

    fn route(mut self, router: Router<S>) -> Self {
        self.router = Some(Arc::new(router));
        self
    }

    fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    fn config(self, cfg: ConfigBuilder) -> Self {
        let loaded = cfg.build::<ActixConfig>().unwrap_or_default();
        self.config(loaded)
    }

    fn build(self) -> ServerResult<Self::Server> {
        Ok(ActixHttpServer {
            state: Arc::new(self.state),
            router: self.router,
            host: self.config.host,
            port: self.config.port,
            middlewares: self.middlewares,
        })
    }
}
