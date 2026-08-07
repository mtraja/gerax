use std::sync::Arc;

use futures_util::{stream::StreamExt, SinkExt};
use gerax_http::ServerResult;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio_tungstenite::accept_async;

use crate::websocket::handler::{WsContext, WsHandler, WsResult, ServerError};
use crate::websocket::message::WsMessage;

pub struct WebSocketServer<State> {
    state: Arc<State>,
    addr: std::net::SocketAddr,
    handler: Arc<dyn WsHandler<State>>,
    shutdown: Arc<(std::sync::atomic::AtomicBool, Notify)>,
    bound_addr: Arc<Mutex<Option<std::net::SocketAddr>>>,
}

impl<State> Clone for WebSocketServer<State> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            addr: self.addr,
            handler: Arc::clone(&self.handler),
            shutdown: Arc::clone(&self.shutdown),
            bound_addr: Arc::clone(&self.bound_addr),
        }
    }
}

impl<State> WebSocketServer<State>
where
    State: Send + Sync + 'static,
{
    pub fn new(
        addr: std::net::SocketAddr,
        state: Arc<State>,
        handler: Arc<dyn WsHandler<State>>,
    ) -> Self {
        Self {
            state,
            addr,
            handler,
            shutdown: Arc::new((
                std::sync::atomic::AtomicBool::new(false),
                Notify::new(),
            )),
            bound_addr: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn addr(&self) -> Option<std::net::SocketAddr> {
        self.bound_addr.lock().await.clone()
    }

    pub async fn run(self) -> ServerResult<()> {
        let listener = TcpListener::bind(&self.addr)
            .await
            .map_err(|e| gerax_http::HttpServerError::InitializationFailed(e.to_string()))?;

        let bound = listener.local_addr().map_err(|e| {
            gerax_http::HttpServerError::InitializationFailed(e.to_string())
        })?;
        *self.bound_addr.lock().await = Some(bound);

        loop {
            tokio::select! {
                _ = self.shutdown.1.notified() => {
                    break;
                }
                accept = listener.accept() => {
                    let (stream, peer_addr) = accept.map_err(|e| gerax_http::HttpServerError::RuntimeError(e.to_string()))?;
                    let state = Arc::clone(&self.state);
                    let handler = Arc::clone(&self.handler);
                    let shutdown = Arc::clone(&self.shutdown);
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(state, handler, peer_addr, stream, shutdown).await {
                            eprintln!("WebSocket connection error: {}", e);
                        }
                    });
                }
            }
        }

        Ok(())
    }

    async fn handle_connection(
        state: Arc<State>,
        handler: Arc<dyn WsHandler<State>>,
        peer_addr: std::net::SocketAddr,
        stream: tokio::net::TcpStream,
        shutdown: Arc<(std::sync::atomic::AtomicBool, Notify)>,
    ) -> WsResult<()> {
        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| ServerError::ConnectionError(e.to_string()))?;

        let (sink, mut stream) = ws_stream.split();

        let (tx, mut rx) = mpsc::channel::<WsMessage>(32);
        let ctx = WsContext::new(state, peer_addr, tx);
        handler.on_open(ctx.clone()).await?;

        let mut sink = sink;
        let send_handler = Arc::clone(&handler);
        let send_ctx = ctx.clone();
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let frame: tungstenite::Message = msg.into();
                if let Err(e) = sink.send(frame).await {
                    let _ = send_handler
                        .on_error(send_ctx.clone(), ServerError::ConnectionError(e.to_string()))
                        .await;
                    break;
                }
            }
        });

        loop {
            if shutdown.0.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = handler
                    .on_close(ctx.clone(), 1000, "Server shutdown".to_string())
                    .await;
                break;
            }

            match stream.next().await {
                Some(Ok(tungstenite::Message::Close(_))) => {
                    let _ = handler
                        .on_close(ctx.clone(), 1000, "Normal closure".to_string())
                        .await;
                    break;
                }
                Some(Ok(msg)) => {
                    let msg: WsMessage = msg.into();
                    if let Err(e) = handler.on_message(ctx.clone(), msg).await {
                        let _ = handler.on_error(ctx.clone(), e).await;
                        break;
                    }
                }
                Some(Err(e)) => {
                    let error = ServerError::ConnectionError(e.to_string());
                    let _ = handler.on_error(ctx.clone(), error).await;
                    break;
                }
                None => {
                    let _ = handler
                        .on_close(ctx.clone(), 1000, "Connection closed".to_string())
                        .await;
                    break;
                }
            }
        }

        send_task.abort();
        Ok(())
    }

    pub async fn stop(&self) -> ServerResult<()> {
        self.shutdown.0.store(true, std::sync::atomic::Ordering::SeqCst);
        self.shutdown.1.notify_one();
        Ok(())
    }
}
