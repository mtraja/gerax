use crate::ServerResult;
use crate::Middleware;
use crate::HttpServer;
use crate::Router;
use crate::ServerConfig;



pub trait HttpServerBuilder<State>: Sized + Send + Sync
where
    State: Send + Sync + 'static,
{
    type Server: HttpServer;

    fn new(state: State) -> Self;

    fn middleware<M>(self, middleware: M) -> Self
    where
        M: Middleware<State>;

    fn route(self, router: Router<State>) -> Self;

    fn config(self, config: ServerConfig) -> Self;

    fn build(self) -> ServerResult<Self::Server>;
}
