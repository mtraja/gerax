use std::sync::Arc;

use futures_util::{stream::StreamExt, SinkExt};
use gerax_http::ServerResult;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio_tungstenite::connect_async;

use crate::websocket::handler::{WsContext, WsHandler, ServerError};
use crate::websocket::message::WsMessage;

pub struct WebSocketClient<State> {
    url: String,
    state: Arc<State>,
    handler: Arc<dyn WsHandler<State>>,
    shutdown: Arc<(std::sync::atomic::AtomicBool, Notify)>,
    sender: Arc<Mutex<Option<mpsc::Sender<WsMessage>>>>,
}

impl<State> WebSocketClient<State>
where
    State: Send + Sync + 'static,
{
    pub fn new(url: String, state: Arc<State>, handler: Arc<dyn WsHandler<State>>) -> Self {
        Self {
            url,
            state,
            handler,
            shutdown: Arc::new((
                std::sync::atomic::AtomicBool::new(false),
                Notify::new(),
            )),
            sender: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn connect(&self) -> ServerResult<()> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| gerax_http::HttpServerError::RuntimeError(e.to_string()))?;

        let (sink, mut stream) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<WsMessage>(32);

        {
            let mut sender_guard = self.sender.lock().await;
            *sender_guard = Some(tx.clone());
        }

        let state = Arc::clone(&self.state);
        let handler = Arc::clone(&self.handler);
        let shutdown = Arc::clone(&self.shutdown);
        let peer_addr = std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
            0,
        );

        let ctx = WsContext::new(state, peer_addr, tx.clone());
        handler.on_open(ctx.clone()).await.map_err(|e| {
            gerax_http::HttpServerError::RuntimeError(e.to_string())
        })?;

        let send_handler = Arc::clone(&handler);
        let send_ctx = ctx.clone();
        let send_task = tokio::spawn(async move {
            let mut sink = sink;
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

        tokio::spawn(async move {
            loop {
                if shutdown.0.load(std::sync::atomic::Ordering::SeqCst) {
                    let _ = handler
                        .on_close(ctx.clone(), 1000, "Client shutdown".to_string())
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
        });

        Ok(())
    }

    pub async fn send(&self, msg: WsMessage) -> ServerResult<()> {
        let sender = self.sender.lock().await;
        if let Some(ref sender) = *sender {
            sender
                .send(msg)
                .await
                .map_err(|e| gerax_http::HttpServerError::RuntimeError(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn close(&self) -> ServerResult<()> {
        self.shutdown.0.store(true, std::sync::atomic::Ordering::SeqCst);
        self.shutdown.1.notify_one();
        Ok(())
    }
}