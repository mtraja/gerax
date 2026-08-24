use gerax_actix::{ActixHttpServer, ActixHttpServerBuilder};
use gerax_http::{HttpServerBuilder, Router, ServerConfig, ServerResult};

use super::HttpRuntime;

/// Runtime HTTP baseado em Actix Web.
#[derive(Debug, Default, Clone, Copy)]
pub struct ActixRuntime;

impl<State> HttpRuntime<State> for ActixRuntime
where
    State: Send + Sync + 'static,
{
    type Server = ActixHttpServer<State>;

    fn build(
        state: State,
        router: Router<State>,
        config: ServerConfig,
    ) -> ServerResult<Self::Server> {
        ActixHttpServerBuilder::from_config(state, config)
            .route(router)
            .build()
    }
}
