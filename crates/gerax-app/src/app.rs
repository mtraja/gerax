use gerax_http::{HttpServer, Router, ServerConfig};

use crate::{AppResult, HttpRuntime};

/// Constrói uma aplicação Gerax a partir do estado compartilhado.
pub struct AppBuilder<State> {
    state: State,
    router: Router<State>,
    server_config: ServerConfig,
}

impl<State> AppBuilder<State>
where
    State: Send + Sync + 'static,
{
    /// Cria um builder com roteador e configuração HTTP padrão.
    pub fn new(state: State) -> Self {
        Self {
            state,
            router: Router::new(),
            server_config: ServerConfig::default(),
        }
    }

    /// Define todas as rotas da aplicação.
    pub fn router(mut self, router: Router<State>) -> Self {
        self.router = router;
        self
    }

    /// Define a configuração de bind do servidor HTTP.
    pub fn server_config(mut self, server_config: ServerConfig) -> Self {
        self.server_config = server_config;
        self
    }

    /// Finaliza a composição da aplicação.
    pub fn build(self) -> App<State> {
        App {
            state: self.state,
            router: self.router,
            server_config: self.server_config,
        }
    }
}

/// Aplicação pronta para ser executada em um runtime HTTP.
pub struct App<State> {
    state: State,
    router: Router<State>,
    server_config: ServerConfig,
}

impl<State> App<State>
where
    State: Send + Sync + 'static,
{
    /// Cria um builder para a aplicação.
    pub fn builder(state: State) -> AppBuilder<State> {
        AppBuilder::new(state)
    }

    /// Inicia a aplicação usando o runtime informado.
    pub async fn run<Runtime>(self) -> AppResult
    where
        Runtime: HttpRuntime<State>,
    {
        let server = Runtime::build(self.state, self.router, self.server_config)?;
        server.run().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_uses_default_server_config() {
        let app = AppBuilder::new(()).build();

        assert_eq!(app.server_config.host, "0.0.0.0");
        assert_eq!(app.server_config.port, 8080);
        assert!(app.router.routes().is_empty());
    }
}
