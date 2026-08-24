use gerax_http::{HttpServer, Router, ServerConfig, ServerResult};

/// Adaptador capaz de construir um servidor HTTP para uma aplicação Gerax.
pub trait HttpRuntime<State>
where
    State: Send + Sync + 'static,
{
    /// Servidor concreto produzido pelo runtime.
    type Server: HttpServer;

    /// Constrói o servidor com o estado, rotas e configuração fornecidos.
    fn build(
        state: State,
        router: Router<State>,
        config: ServerConfig,
    ) -> ServerResult<Self::Server>;
}

#[cfg(feature = "actix")]
mod actix;

#[cfg(feature = "actix")]
pub use actix::ActixRuntime;
