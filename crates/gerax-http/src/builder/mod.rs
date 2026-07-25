use crate::ServerResult;
use crate::Middleware;
use crate::HttpServer;
use crate::Router;
use gerax_config::builder::ConfigBuilder;



pub trait HttpServerBuilder<State>: Sized + Send + Sync
where
    State: Send + Sync + 'static,
{
    type Server: HttpServer;

    fn new(state: State) -> Self;

    fn middleware<M>(self, middleware: M) -> Self
    where
        M: Middleware;

    fn route(self, router: Router<State>) -> Self;

    fn config(self, cfg: ConfigBuilder) -> Self;

    fn build(self) -> ServerResult<Self::Server>;
}
