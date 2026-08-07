use std::sync::Arc;

use gerax_websocket::{
    ServerError, WebSocketServer, WsContext, WsHandler, WsMessage,
};

struct EchoHandler;

#[async_trait::async_trait]
impl WsHandler<()> for EchoHandler {
    async fn on_message(&self, ctx: WsContext<()>, msg: WsMessage) -> Result<(), ServerError> {
        println!("Received: {}", msg);
        ctx.send(msg).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let handler = Arc::new(EchoHandler);
    let server = WebSocketServer::new(
        "127.0.0.1:8080".parse().unwrap(),
        Arc::new(()),
        handler,
    );

    println!("WebSocket echo server running on ws://127.0.0.1:8080");
    server.run().await.unwrap();
}
