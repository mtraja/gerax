use std::sync::Arc;

use async_trait::async_trait;
use gerax_http::routing::Extensions;
use thiserror::Error;

use crate::websocket::message::WsMessage;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Erro no handler: {0}")]
    HandlerError(String),

    #[error("Erro de conexão: {0}")]
    ConnectionError(String),
}

pub type WsResult<T = ()> = Result<T, ServerError>;

pub struct WsContext<State> {
    pub state: Arc<State>,
    pub peer_addr: std::net::SocketAddr,
    pub extensions: Extensions,
    pub sender: tokio::sync::mpsc::Sender<WsMessage>,
}

impl<State> Clone for WsContext<State> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            peer_addr: self.peer_addr,
            extensions: self.extensions.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<State> WsContext<State> {
    pub fn new(
        state: Arc<State>,
        peer_addr: std::net::SocketAddr,
        sender: tokio::sync::mpsc::Sender<WsMessage>,
    ) -> Self {
        Self {
            state,
            peer_addr,
            extensions: Extensions::new(),
            sender,
        }
    }

    pub fn state(&self) -> Arc<State> {
        Arc::clone(&self.state)
    }

    pub fn peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub async fn send(&self, msg: WsMessage) -> WsResult<()> {
        self.sender
            .send(msg)
            .await
            .map_err(|e| ServerError::ConnectionError(e.to_string()))
    }
}

#[async_trait]
pub trait WsHandler<State>: Send + Sync + 'static
where
    State: Send + Sync + 'static,
{
    async fn on_open(&self, _ctx: WsContext<State>) -> WsResult<()> {
        Ok(())
    }

    async fn on_message(&self, ctx: WsContext<State>, msg: WsMessage) -> WsResult<()>;

    async fn on_close(
        &self,
        _ctx: WsContext<State>,
        _code: u16,
        _reason: String,
    ) -> WsResult<()> {
        Ok(())
    }

    async fn on_error(&self, _ctx: WsContext<State>, _error: ServerError) -> WsResult<()> {
        Ok(())
    }
}

#[async_trait]
impl<State, F, Fut> WsHandler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(WsContext<State>, WsMessage) -> Fut,
    Fut: std::future::Future<Output = WsResult<()>> + Send,
{
    async fn on_message(&self, ctx: WsContext<State>, msg: WsMessage) -> WsResult<()> {
        (self)(ctx, msg).await
    }
}
