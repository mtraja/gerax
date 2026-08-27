use gerax_http::{HttpServer, Router, ServerConfig, CorsConfig};

use crate::{AppResult, HttpRuntime};

/// Constrói uma aplicação Gerax a partir do estado compartilhado.
pub struct AppBuilder<State> {
    state: State,
    router: Router<State>,
    server_config: ServerConfig,
    cors_config: Option<CorsConfig>,
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
            cors_config: None,
        }
    }

    /// Cria um builder a partir de uma configuração agregada (`AppConfig`).
    #[cfg(feature = "config")]
    pub fn from_config(state: State, config: crate::config::AppConfig) -> Self {
        Self {
            state,
            router: Router::new(),
            server_config: config.server,
            cors_config: None,
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

    /// Habilita CORS com a configuração informada.
    pub fn cors(mut self, cors_config: CorsConfig) -> Self {
        self.cors_config = Some(cors_config);
        self
    }

    /// Aplica configuração agregada (`AppConfig`), sobrescrevendo o `server_config`.
    #[cfg(feature = "config")]
    pub fn config(self, config: crate::config::AppConfig) -> Self {
        Self {
            server_config: config.server,
            ..self
        }
    }

    /// Finaliza a composição da aplicação.
    pub fn build(self) -> App<State> {
        App {
            state: self.state,
            router: self.router,
            server_config: self.server_config,
            cors_config: self.cors_config,
        }
    }
}

/// Aplicação pronta para ser executada em um runtime HTTP.
pub struct App<State> {
    state: State,
    router: Router<State>,
    server_config: ServerConfig,
    cors_config: Option<CorsConfig>,
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
        let server = Runtime::build(self.state, self.router, self.server_config, self.cors_config)?;
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
        assert!(app.cors_config.is_none());
    }

    #[cfg(feature = "config")]
    #[test]
    fn from_config_applies_server_config() {
        let config = crate::config::AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 3000,
            },
            #[cfg(feature = "db")]
            database: gerax_db::DatabaseConfig::default(),
        };

        let app = AppBuilder::from_config((), config).build();

        assert_eq!(app.server_config.host, "127.0.0.1");
        assert_eq!(app.server_config.port, 3000);
    }
}
