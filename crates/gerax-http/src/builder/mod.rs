use crate::error::ServerResult;
use crate::middleware::Middleware;
use crate::server::HttpServer;
use crate::Router;



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

    fn config(self, ) -> Self;

    fn build(self) -> ServerResult<Self::Server>;
}
