use std::sync::Arc;

use tokio::sync::Mutex;

use gerax_websocket::{
    ServerError, WebSocketClient, WebSocketServer, WsContext, WsHandler, WsMessage,
};

#[derive(Clone)]
struct SharedReceived {
    inner: Arc<Mutex<Vec<WsMessage>>>,
}

impl SharedReceived {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn push(&self, msg: WsMessage) {
        self.inner.lock().await.push(msg);
    }

    async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }
}

struct EchoHandler {
    received: SharedReceived,
}

impl EchoHandler {
    fn new(received: SharedReceived) -> Self {
        Self { received }
    }
}

#[async_trait::async_trait]
impl WsHandler<()> for EchoHandler {
    async fn on_message(&self, ctx: WsContext<()>, msg: WsMessage) -> Result<(), ServerError> {
        self.received.push(msg.clone()).await;
        ctx.send(msg).await?;
        Ok(())
    }
}

#[tokio::test]
async fn echo_server_basic() {
    let server_received = SharedReceived::new();
    let server_handler = Arc::new(EchoHandler::new(server_received.clone()));

    let server = WebSocketServer::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::new(()),
        server_handler,
    );

    let server_clone = server.clone();
    let server_task = tokio::spawn(async move {
        server_clone.run().await
    });

    let bound_addr = loop {
        if let Some(addr) = server.addr().await {
            break addr;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    };
    let url = format!("ws://{}", bound_addr);
    let client_received = SharedReceived::new();
    let client_handler = Arc::new(EchoHandler::new(client_received.clone()));

    let client = WebSocketClient::new(url, Arc::new(()), client_handler);
    
    for attempt in 0..10 {
        if client.connect().await.is_ok() {
            break;
        }
        if attempt == 9 {
            panic!("Failed to connect after 10 attempts");
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    client
        .send(WsMessage::Text("hello".to_string()))
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    assert!(server_received.len().await >= 1);
    assert!(client_received.len().await >= 1);

    server.stop().await.unwrap();
    let _ = server_task.await;
}

#[tokio::test]
async fn multiple_connections() {
    let server_received = SharedReceived::new();
    let server_handler = Arc::new(EchoHandler::new(server_received));

    let server = WebSocketServer::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::new(()),
        server_handler,
    );

    let server_clone = server.clone();
    let server_task = tokio::spawn(async move {
        server_clone.run().await
    });

    let bound_addr = loop {
        if let Some(addr) = server.addr().await {
            break addr;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    };
    let url = format!("ws://{}", bound_addr);

    let client1_handler = Arc::new(EchoHandler::new(SharedReceived::new()));
    let client2_handler = Arc::new(EchoHandler::new(SharedReceived::new()));

    let client1 = WebSocketClient::new(url.clone(), Arc::new(()), client1_handler);
    let client2 = WebSocketClient::new(url.clone(), Arc::new(()), client2_handler);

    for attempt in 0..10 {
        if client1.connect().await.is_ok() && client2.connect().await.is_ok() {
            break;
        }
        if attempt == 9 {
            panic!("Failed to connect clients after 10 attempts");
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    server.stop().await.unwrap();
    let _ = server_task.await;
}

#[tokio::test]
async fn shutdown_is_clean() {
    let server_handler = Arc::new(EchoHandler::new(SharedReceived::new()));

    let server = WebSocketServer::new(
        "127.0.0.1:0".parse().unwrap(),
        Arc::new(()),
        server_handler,
    );

    let server_clone = server.clone();
    let server_task = tokio::spawn(async move {
        server_clone.run().await
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    server.stop().await.unwrap();

    let result = server_task.await;
    assert!(result.is_ok());
}
