//! Referência canônica do módulo `gerax-http`.
//!
//! Este arquivo é o exemplo "fonte da verdade" que a skill usa para gerar
//! código consistente. Ele não depende de nenhum framework HTTP concreto
//! (axum, actix-web, warp, etc.) — a escolha da tecnologia é sempre da
//! aplicação, nunca deste módulo.
//!
//! Arquitetura: Hexagonal / Ports & Adapters.
//! - Este arquivo representa a "porta" (port): traits abstratas.
//! - Um crate/módulo separado (ex: `gerax-http-axum`) seria o "adaptador"
//!   (adapter), implementando a porta usando um framework real.
//! - A regra de dependência é sempre porta <- adaptador, nunca o contrário.

use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum GeraxHttpError {
    Bind(String),
    Runtime(String),
    Shutdown(String),
    Config(String),
}

impl fmt::Display for GeraxHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(msg) => write!(f, "falha ao iniciar (bind): {msg}"),
            Self::Runtime(msg) => write!(f, "falha em tempo de execução: {msg}"),
            Self::Shutdown(msg) => write!(f, "falha ao encerrar: {msg}"),
            Self::Config(msg) => write!(f, "configuração inválida: {msg}"),
        }
    }
}

impl StdError for GeraxHttpError {}

#[async_trait::async_trait]
pub trait HttpServer<S>: Send
where
    S: Clone + Send + Sync + 'static,
{
    async fn listen(&mut self, state: S) -> Result<(), GeraxHttpError>;

    fn configure_routes(&mut self, state: &S) {
        let _ = state;
    }
}

pub trait HttpServerBuilder<S>: Sized
where
    S: Clone + Send + Sync + 'static,
{
    type Server: HttpServer<S>;

    fn with_middleware(self, name: impl Into<String>) -> Self;

    fn with_option(self, key: impl Into<String>, value: impl Into<String>) -> Self;

    fn build(self) -> Self::Server;
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NoopHttpServer {
    pub middlewares: Vec<String>,
    pub options: Vec<(String, String)>,
}

#[async_trait::async_trait]
impl<S> HttpServer<S> for NoopHttpServer
where
    S: Clone + Send + Sync + 'static,
{
    async fn listen(&mut self, state: S) -> Result<(), GeraxHttpError> {
        self.configure_routes(&state);
        Ok(())
    }
}

pub struct NoopHttpServerBuilder {
    server: NoopHttpServer,
}

impl NoopHttpServerBuilder {
    pub fn new() -> Self {
        Self {
            server: NoopHttpServer::default(),
        }
    }
}

impl<S> HttpServerBuilder<S> for NoopHttpServerBuilder
where
    S: Clone + Send + Sync + 'static,
{
    type Server = NoopHttpServer;

    fn with_middleware(mut self, name: impl Into<String>) -> Self {
        self.server.middlewares.push(name.into());
        self
    }

    fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.server.options.push((key.into(), value.into()));
        self
    }

    fn build(self) -> Self::Server {
        self.server
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct AppState {
        counter: i32,
        name: String,
    }

    #[test]
    fn default_configure_routes_does_not_mutate_state() {
        let state_before = AppState {
            counter: 42,
            name: "gerax".to_string(),
        };
        let state_after = state_before.clone();

        let mut server = NoopHttpServer::default();
        HttpServer::<AppState>::configure_routes(&mut server, &state_before);

        assert_eq!(state_before, state_after, "estado não deve ser alterado pelo no-op padrão");
    }

    #[test]
    fn builder_allows_chained_configuration() {
        let server = <NoopHttpServerBuilder as HttpServerBuilder<AppState>>::build(
            HttpServerBuilder::<AppState>::with_option(
                HttpServerBuilder::<AppState>::with_middleware(
                    NoopHttpServerBuilder::new(),
                    "logger",
                ),
                "port",
                "8080",
            ),
        );
        assert_eq!(server.middlewares, vec!["logger".to_string()]);
        assert_eq!(server.options, vec![("port".to_string(), "8080".to_string())]);
    }

    #[tokio::test]
    async fn listen_completes_with_shared_state() {
        let state = AppState {
            counter: 1,
            name: "test".to_string(),
        };
        let mut server = NoopHttpServer::default();
        let result = server.listen(state).await;
        assert!(result.is_ok());
    }
}