use std::sync::Arc;

use gerax_config::builder::ConfigBuilder;

use crate::ActixHttpServer;

use gerax_http::{HttpServerBuilder, Middleware, Router, ServerResult, ServerConfig};

/// Builder para construção fluida de servidores HTTP com Actix Web.
pub struct ActixHttpServerBuilder<S> {
    state: S,
    router: Option<Arc<Router<S>>>,
    config: ServerConfig,
    middlewares: Vec<Arc<dyn Middleware<S>>>,
}

impl<S> ActixHttpServerBuilder<S>
where
    S: Send + Sync + 'static,
{
    /// Cria um novo builder a partir do estado compartilhado.
    pub fn new(state: S) -> Self {
        Self {
            state,
            router: None,
            config: ServerConfig::default(),
            middlewares: Vec::new(),
        }
    }

    /// Cria um novo builder a partir do estado compartilhado e configuração inicial.
    pub fn from_config(state: S, config: ServerConfig) -> Self {
        Self {
            state,
            router: None,
            config,
            middlewares: Vec::new(),
        }
    }

    /// Aplica uma configuração de servidor ao builder.
    pub fn config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Define o roteador do servidor.
    pub fn route(mut self, router: Router<S>) -> Self {
        self.router = Some(Arc::new(router));
        self
    }

    /// Adiciona um middleware ao servidor.
    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<S>,
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

    /// Cria um novo builder para o estado informado.
    fn new(state: S) -> Self {
        Self::new(state)
    }

    /// Define o roteador do servidor.
    fn route(mut self, router: Router<S>) -> Self {
        self.router = Some(Arc::new(router));
        self
    }

    /// Adiciona um middleware ao servidor.
    fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<S>,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    /// Aplica configuração carregada via `gerax-config`.
    fn config(self, cfg: ConfigBuilder) -> Self {
        let loaded = cfg.build::<ServerConfig>().unwrap_or_default();
        self.config(loaded)
    }

    /// Constrói a instância de servidor Actix pronta para rodar.
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
