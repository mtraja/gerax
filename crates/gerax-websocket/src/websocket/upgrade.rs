use std::sync::Arc;

use async_trait::async_trait;
use gerax_http::routing::{Context, Response};
use gerax_http::ServerResult;

use crate::websocket::handler::WsHandler;

#[derive(Clone)]
pub struct WsHandlerExtension<State> {
    pub handler: Arc<dyn WsHandler<State>>,
}

impl<State> WsHandlerExtension<State> {
    pub fn new(handler: Arc<dyn WsHandler<State>>) -> Self {
        Self { handler }
    }
}

pub struct WsUpgradeHandler<State> {
    handler: Arc<dyn WsHandler<State>>,
}

impl<State> WsUpgradeHandler<State> {
    pub fn new(handler: Arc<dyn WsHandler<State>>) -> Self {
        Self { handler }
    }

    pub fn handler(&self) -> Arc<dyn WsHandler<State>> {
        Arc::clone(&self.handler)
    }
}

#[async_trait]
impl<State> gerax_http::routing::Handler<State> for WsUpgradeHandler<State>
where
    State: Send + Sync + 'static,
{
    async fn call(&self, mut context: Context<State>) -> ServerResult<Response> {
        let headers = context.request.headers();

        let is_ws_upgrade = headers
            .get("upgrade")
            .map(|v| v.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false)
            && headers
                .get("connection")
                .map(|v| v.eq_ignore_ascii_case("upgrade"))
                .unwrap_or(false);

        if !is_ws_upgrade {
            return Ok(Response {
                status: 400,
                body: b"Bad Request: Expected WebSocket upgrade".to_vec(),
                ..Default::default()
            });
        }

        context
            .extensions_mut()
            .insert(WsHandlerExtension::new(Arc::clone(&self.handler)));

        Ok(Response {
            status: 101,
            body: Vec::new(),
            ..Default::default()
        })
    }
}
