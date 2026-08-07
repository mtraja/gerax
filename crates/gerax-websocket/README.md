# gerax-websocket

Suporte a WebSocket no ecossistema Gerax.

## Dependência

```toml
gerax-websocket = { path = "../crates/gerax-websocket" }
```

No `Cargo.toml` da workspace, a feature `websocket` já está disponível na facade `gerax`:

```toml
gerax = { path = "../crates/gerax", features = ["websocket"] }
```

## Visão Geral

O `gerax-websocket` fornece abstrações assíncronas para servidor e cliente WebSocket genéricos, seguindo os padrões do ecossistema Gerax:

- Estado genérico `State` propagado em `WsContext<State>`
- Trait-based design com `WsHandler<State>` para processamento de mensagens
- Integração com `gerax-http` via `WsUpgradeHandler` para upgrade HTTP-to-WebSocket
- Integração com `gerax-db` via `RepositoryResolver` para acesso a repositórios nos handlers
- Bridge de subscriptions GraphQL via protocolo WebSocket (`gerax-graphql`)

```
WebSocketServer<State> -> WsHandler<State> -> WsContext<State>
WebSocketClient<State> -> WsHandler<State> -> WsContext<State>
WsUpgradeHandler<State> -> gerax_http::Handler<State>
```

## API

### WsMessage

Mensagem WebSocket canônica do crate.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
    Close(Option<CloseFrame>),
    Ping,
    Pong,
}
```

Conversões de/para `tungstenite::Message` são implementadas automaticamente.

### CloseFrame

Frame de fechamento.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloseFrame {
    pub code: u16,
    pub reason: String,
}
```

---

### ServerError

Erros do crate.

```rust
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Erro no handler: {0}")]
    HandlerError(String),

    #[error("Erro de conexão: {0}")]
    ConnectionError(String),
}
```

`WsResult<T = ()> = Result<T, ServerError>`

---

### WsContext<State>

Contexto de uma conexão WebSocket.

```rust
pub struct WsContext<State> {
    pub state: Arc<State>,
    pub peer_addr: std::net::SocketAddr,
    pub extensions: Extensions,
    pub sender: tokio::sync::mpsc::Sender<WsMessage>,
}
```

Métodos:
- `new(state, peer_addr, sender) -> Self`
- `state(&self) -> Arc<State>`
- `peer_addr(&self) -> SocketAddr`
- `extensions(&self) -> &Extensions`
- `extensions_mut(&mut self) -> &mut Extensions`
- `send(&self, msg: WsMessage) -> WsResult<()>` — envia mensagem ao cliente

---

### WsHandler<State>

Trait para handlers WebSocket.

```rust
#[async_trait]
pub trait WsHandler<State>: Send + Sync + 'static
where
    State: Send + Sync + 'static,
{
    async fn on_open(&self, _ctx: WsContext<State>) -> WsResult<()> { Ok(()) }

    async fn on_message(&self, ctx: WsContext<State>, msg: WsMessage) -> WsResult<()>;

    async fn on_close(
        &self,
        _ctx: WsContext<State>,
        _code: u16,
        _reason: String,
    ) -> WsResult<()> { Ok(()) }

    async fn on_error(&self, _ctx: WsContext<State>, _error: ServerError) -> WsResult<()> { Ok(()) }
}
```

Blanket implementado para closures `Fn(WsContext<State>, WsMessage) -> Fut`.

---

### WebSocketServer<State>

Servidor WebSocket genérico.

```rust
pub struct WebSocketServer<State> {
    // campos privados
}

impl<State> WebSocketServer<State>
where
    State: Send + Sync + 'static,
{
    pub fn new(
        addr: SocketAddr,
        state: Arc<State>,
        handler: Arc<dyn WsHandler<State>>,
    ) -> Self;

    pub async fn addr(&self) -> Option<SocketAddr>;
    pub async fn run(self) -> ServerResult<()>;
    pub async fn stop(&self) -> ServerResult<()>;
}
```

- `new`: cria o servidor. Use `127.0.0.1:0` para porta aleatória.
- `addr`: retorna o endereço efetivamente bound após `run()`.
- `run`: aceita conexões TCP, faz upgrade WebSocket e spawna tasks por conexão.
- `stop`: sinaliza shutdown limpo para todas as conexões.

---

### WebSocketClient<State>

Cliente WebSocket genérico.

```rust
pub struct WebSocketClient<State> {
    // campos privados
}

impl<State> WebSocketClient<State>
where
    State: Send + Sync + 'static,
{
    pub fn new(
        url: String,
        state: Arc<State>,
        handler: Arc<dyn WsHandler<State>>,
    ) -> Self;

    pub async fn connect(&self) -> ServerResult<()>;
    pub async fn send(&self, msg: WsMessage) -> ServerResult<()>;
    pub async fn close(&self) -> ServerResult<()>;
}
```

- `new`: cria o cliente com URL do servidor.
- `connect`: estabelece conexão WebSocket e spawna tasks de leitura/escrita.
- `send`: envia mensagem ao servidor (disponível apenas após `connect()`).
- `close`: sinaliza shutdown.

---

### WsUpgradeHandler<State>

Adapta `WsHandler<State>` para `gerax_http::routing::Handler<State>`, permitindo registrar rotas WebSocket no roteador HTTP.

```rust
pub struct WsUpgradeHandler<State> {
    // campos privados
}

impl<State> WsUpgradeHandler<State> {
    pub fn new(handler: Arc<dyn WsHandler<State>>) -> Self;
    pub fn handler(&self) -> Arc<dyn WsHandler<State>>;
}
```

Retorna `101 Switching Protocols` quando os headers `Upgrade: websocket` e `Connection: Upgrade` estão presentes; caso contrário, retorna `400`.

---

### RepositoryResolver / WsRepository

Integração com `gerax-db` para injetar e acessar repositórios nos handlers.

```rust
#[derive(Clone)]
pub struct WsRepository<T>(pub Arc<dyn Repository<T> + Send + Sync>);

impl<T> WsRepository<T> {
    pub fn new(repo: Arc<dyn Repository<T> + Send + Sync>) -> Self;
    pub fn inner(&self) -> Arc<dyn Repository<T> + Send + Sync>;
}

pub trait RepositoryResolver<State> {
    fn register_repository<T: Entity + Send + Sync + 'static>(&self, repository: WsRepository<T>);
    fn repository<T: Entity + Send + Sync + 'static>(&self) -> Result<WsRepository<T>, WsError>;
}
```

Implementado para `WsContext<State>`.

---

### GraphQL WebSocket Protocol

Mensagens para bridge de subscriptions GraphQL.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum GraphQLClientMessage {
    ConnectionInit,
    Start { id: String, payload: GraphQLStartPayload },
    Stop { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GraphQLServerMessage {
    ConnectionAck,
    Data { id: String, payload: GraphQLDataPayload },
    Error { id: String, payload: GraphQLErrorPayload },
    Complete { id: String },
}
```

---

## Exemplos

### Echo Server

```rust
use std::sync::Arc;

use gerax_websocket::{
    handler::WsHandler,
    websocket::{WebSocketServer, WsContext, WsMessage},
    ServerError,
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
```

### Cliente WebSocket

```rust
use std::sync::Arc;

use gerax_websocket::{
    handler::WsHandler,
    websocket::{WebSocketClient, WsContext, WsMessage},
    ServerError,
};

struct PrintHandler;

#[async_trait::async_trait]
impl WsHandler<()> for PrintHandler {
    async fn on_message(&self, ctx: WsContext<()>, msg: WsMessage) -> Result<(), ServerError> {
        println!("Client received: {}", msg);
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let handler = Arc::new(PrintHandler);
    let client = WebSocketClient::new(
        "ws://127.0.0.1:8080".to_string(),
        Arc::new(()),
        handler,
    );

    client.connect().await.unwrap();
    client.send(WsMessage::Text("hello".to_string())).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    client.close().await.unwrap();
}
```

### Integração com gerax-http (Upgrade)

```rust
use std::sync::Arc;

use gerax_http::routing::Router;
use gerax_websocket::{
    handler::WsHandler,
    upgrade::WsUpgradeHandler,
    websocket::{WebSocketServer, WsContext, WsMessage},
    ServerError,
};

struct HttpHandler;

#[async_trait::async_trait]
impl WsHandler<()> for HttpHandler {
    async fn on_message(&self, ctx: WsContext<()>, msg: WsMessage) -> Result<(), ServerError> {
        ctx.send(msg).await?;
        Ok(())
    }
}

// Registrar rota de upgrade no roteador HTTP
let router = Router::new()
    .route("/ws", WsUpgradeHandler::new(Arc::new(HttpHandler)));
```

### Integração com gerax-db (Repository)

```rust
use std::sync::Arc;

use gerax_core::Entity;
use gerax_db::Repository;
use gerax_websocket::{
    handler::WsHandler,
    repository::{RepositoryResolver, WsRepository},
    websocket::{WsContext, WsMessage},
    ServerError,
};

struct User {
    id: String,
    name: String,
}

impl Entity for User {
    fn collection_name() -> &'static str {
        "users"
    }
}

struct DbHandler {
    user_repo: WsRepository<User>,
}

#[async_trait::async_trait]
impl WsHandler<()> for DbHandler {
    async fn on_message(&self, ctx: WsContext<()>, msg: WsMessage) -> Result<(), ServerError> {
        let repo = ctx.repository::<User>()?;
        let users = repo.inner().find_all().await?;
        // ...
        Ok(())
    }
}
```

### Execução

```bash
# Testes
cargo test -p gerax-websocket

# Exemplo echo server
cargo run --example echo_server -p gerax-websocket
```
