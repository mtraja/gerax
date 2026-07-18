use crate::server::HttpServer;

pub trait HttpServerBuilder<S>: Sized
where
    S: Clone + Send + Sync + 'static,
{
    type Server: HttpServer<S>;

    /// Registra um middleware a ser aplicado antes da inicialização.
    fn with_middleware(self, name: impl Into<String>) -> Self;

    /// Define uma opção de configuração (porta, timeout, etc.) antes de
    /// rodar o servidor.
    fn with_option(self, key: impl Into<String>, value: impl Into<String>) -> Self;

    /// Constrói a implementação concreta de `HttpServer`, pronta para
    /// receber o estado compartilhado via `listen`.
    fn build(self) -> Self::Server;
}
