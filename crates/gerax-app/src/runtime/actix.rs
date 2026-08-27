use gerax_actix::{ActixHttpServer, ActixHttpServerBuilder};
use gerax_http::{HttpServerBuilder, Router, ServerConfig, ServerResult, CorsConfig};

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
        cors_config: Option<CorsConfig>,
    ) -> ServerResult<Self::Server> {
        let mut builder = ActixHttpServerBuilder::from_config(state, config)
            .route(router);

        if let Some(cors_config) = cors_config {
            builder = builder.cors(cors_config);
        }

        builder.build()
    }
}
