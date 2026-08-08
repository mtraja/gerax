# gerax-rpc

Abstrações RPC protocol-agnósticas para o ecossistema Gerax.

## Dependências

```toml
gerax-rpc = { path = "../crates/gerax-rpc" }
```

## Features

Nenhuma. Este crate não possui dependências opcionais ou features condicionais.

## Visão Geral

O `gerax-rpc` define contratos, modelos e traits para comunicação RPC sem acoplamento
a protocolo, serialização ou runtime assíncrono. Ele é a base reutilizada por todas
as implementações RPC do Gerax (`gerax-grpc`, `gerax-capnp`).

```
RpcClient ──► RpcTransport ──► RpcServer ──► RpcService ──► RpcMethod
                   │
              RpcRequest<T> ──► RpcContext ──► RpcMetadata
              RpcResponse<T> ──► RpcStatus  ──► RpcExtensions
```

## API

### RpcRequest\<T\>

Requisição RPC genérica, independente de protocolo e serialização.

```rust
use gerax_rpc::{RpcRequest, RpcMetadata, RpcContext};

let request = RpcRequest::new(b"payload".to_vec())
    .with_metadata(RpcMetadata::new())
    .with_context(RpcContext::new());
```

Campos públicos:

- `payload: T` — dados da requisição
- `metadata: RpcMetadata` — metadados (headers, tracing, etc.)
- `context: RpcContext` — contexto de execução

Métodos:

- `RpcRequest::new(payload: T) -> Self` — cria uma nova requisição
- `with_metadata(self, metadata: RpcMetadata) -> Self` — define os metadados
- `with_context(self, context: RpcContext) -> Self` — define o contexto

#### `RpcRequestBuilder<T>`

Builder fluente para `RpcRequest<T>`.

```rust
RpcRequestBuilder::new(payload)
    .insert_metadata("key", "value")
    .context(ctx)
    .build()
```

Métodos:

- `new(payload: T) -> Self` — cria um builder com payload
- `metadata(self, metadata: RpcMetadata) -> Self` — define os metadados
- `insert_metadata(self, key, value) -> Self` — insere um metadado
- `context(self, context: RpcContext) -> Self` — define o contexto
- `build(self) -> RpcRequest<T>` — constrói a requisição

---

### RpcResponse\<T\>

Resposta RPC genérica, independente de protocolo e serialização.

```rust
use gerax_rpc::{RpcResponse, RpcStatus};

let ok = RpcResponse::success("pong".to_string());
let err = RpcResponse::error(RpcStatus::NotFound, "missing");
```

Campos públicos:

- `payload: Option<T>` — dados da resposta
- `status: RpcStatus` — status da resposta
- `message: Option<String>` — mensagem de erro (se houver)
- `metadata: RpcMetadata` — metadados (headers, etc.)

Métodos:

- `RpcResponse::success(payload: T) -> Self` — cria uma resposta de sucesso
- `RpcResponse::error(status: RpcStatus, message: impl Into<String>) -> Self` — cria uma resposta de erro
- `with_metadata(self, metadata: RpcMetadata) -> Self` — define os metadados
- `is_error(&self) -> bool` — verifica se a resposta é um erro

#### `RpcResponseBuilder<T>`

Builder fluente para `RpcResponse<T>`.

```rust
RpcResponseBuilder::<String>::new()
    .status(RpcStatus::AlreadyExists)
    .message("dup")
    .build()
```

Métodos:

- `new() -> Self` — cria um builder com status `Ok`
- `payload(self, payload: T) -> Self` — define o payload
- `status(self, status: RpcStatus) -> Self` — define o status
- `message(self, message: impl Into<String>) -> Self` — define a mensagem
- `metadata(self, metadata: RpcMetadata) -> Self` — define os metadados
- `build(self) -> RpcResponse<T>` — constrói a resposta

---

### RpcStatus

Status de resposta RPC. Segue a semântica dos gRPC status codes, mas é independente de protocolo.

```rust
use gerax_rpc::RpcStatus;

let status = RpcStatus::NotFound;
assert_eq!(status.code(), 5);
assert_eq!(status.description(), "Not Found");
```

Variantes:

`Ok`, `Cancelled`, `Unknown`, `InvalidArgument`, `DeadlineExceeded`, `NotFound`,
`AlreadyExists`, `PermissionDenied`, `ResourceExhausted`, `FailedPrecondition`,
`Aborted`, `OutOfRange`, `Unimplemented`, `Internal`, `Unavailable`, `Unauthenticated`

Métodos:

- `code(&self) -> i32` — retorna o código numérico do status
- `description(&self) -> &'static str` — retorna a descrição textual do status

Conversões:

- `From<i32> for RpcStatus` — converte código numérico em `RpcStatus`

---

### RpcMetadata

Metadados RPC (chave-valor). Usado para headers, contexto de tracing e autenticação.

```rust
use gerax_rpc::RpcMetadata;

let mut meta = RpcMetadata::new();
meta.insert("authorization", "Bearer token");
meta.insert("trace-id", "abc123");

assert!(meta.contains_key("trace-id"));
```

Campos públicos: nenhum (acesso via métodos).

Métodos:

- `new() -> Self` — cria um `RpcMetadata` vazio
- `insert(&mut self, key: impl Into<String>, value: impl Into<String>)` — insere um par chave-valor
- `get(&self, key: &str) -> Option<&String>` — obtém o valor de uma chave
- `remove(&mut self, key: &str) -> Option<String>` — remove uma chave
- `contains_key(&self, key: &str) -> bool` — verifica se contém uma chave
- `iter(&self) -> impl Iterator<Item = (&String, &String)>` — itera sobre os pares chave-valor

Conversões:

- `From<HashMap<String, String>> for RpcMetadata`

#### `RpcMetadataBuilder`

Builder fluente para `RpcMetadata`.

```rust
RpcMetadataBuilder::new()
    .insert("authorization", "Bearer token")
    .insert("trace-id", "abc123")
    .build()
```

Métodos:

- `new() -> Self` — cria um builder vazio
- `insert(self, key, value) -> Self` — insere um par chave-valor
- `build(self) -> RpcMetadata` — constrói o metadado

---

### RpcContext

Contexto de execução RPC. Carrega metadados, deadline e trace-id durante o processamento de uma requisição.

```rust
use gerax_rpc::{RpcContext, RpcMetadata};
use std::time::Duration;

let ctx = RpcContext::new()
    .with_metadata(RpcMetadata::new())
    .with_deadline(Duration::from_secs(30))
    .with_trace_id("abc-123");
```

Campos públicos:

- `metadata: RpcMetadata` — metadados da requisição
- `deadline: Option<Duration>` — tempo limite para a requisição
- `trace_id: Option<String>` — ID de correlação/tracing

Métodos:

- `new() -> Self` — cria um `RpcContext` vazio
- `with_metadata(self, metadata: RpcMetadata) -> Self` — define os metadados
- `with_deadline(self, deadline: Duration) -> Self` — define o tempo limite
- `with_trace_id(self, trace_id: impl Into<String>) -> Self` — define o ID de tracing

#### `RpcContextBuilder`

Builder fluente para `RpcContext`.

```rust
RpcContextBuilder::new()
    .deadline(Duration::from_secs(30))
    .trace_id("abc-123")
    .build()
```

Métodos:

- `new() -> Self` — cria um builder vazio
- `metadata(self, metadata: RpcMetadata) -> Self` — define os metadados
- `deadline(self, deadline: Duration) -> Self` — define o tempo limite
- `trace_id(self, trace_id: impl Into<String>) -> Self` — define o ID de tracing
- `build(self) -> RpcContext` — constrói o contexto

---

### RpcExtensions

Extensões opcionais tipadas para RPC. Permite anexar dados tipados sem acoplamento ao protocolo.

```rust
use gerax_rpc::RpcExtensions;

let mut ext = RpcExtensions::new();
ext.insert(42u32);

assert_eq!(ext.get::<u32>(), Some(&42));
```

Métodos:

- `new() -> Self` — cria um `RpcExtensions` vazio
- `insert<T: Send + Sync + 'static>(&mut self, value: T)` — insere uma extensão tipada
- `get<T: Send + Sync + 'static>(&self) -> Option<&T>` — obtém uma extensão tipada
- `remove<T: Send + Sync + 'static>(&mut self) -> Option<T>` — remove uma extensão tipada

---

### RpcMethod

Trait que representa um método RPC invocável.

```rust
pub trait RpcMethod: Send + Sync + 'static {
    type Request;
    type Response;

    fn name(&self) -> &str;
    async fn call(&self, request: RpcRequest<Self::Request>) -> RpcResponse<Self::Response>;
}
```

Tipos associados:

- `Request` — tipo da requisição
- `Response` — tipo da resposta

Métodos obrigatórios:

- `name(&self) -> &str` — nome do método (usado para roteamento/despacho)
- `call(&self, request: RpcRequest<Self::Request>) -> RpcResponse<Self::Response>` — invoca o método

---

### RpcService

Trait que representa um serviço RPC. Agrupa métodos e faz o despacho de requisições.

```rust
pub trait RpcService: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn register<M: RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + 'static>(&mut self, method: M);
    async fn call(&self, method: &str, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>>;
}
```

Métodos obrigatórios:

- `name(&self) -> &str` — nome do serviço
- `register<M: RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + 'static>(&mut self, method: M)` — registra um método
- `call(&self, method: &str, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>>` — invoca um método pelo nome

---

### SimpleRpcService

Implementação de `RpcService` com despacho real de métodos.

```rust
use gerax_rpc::{RpcMethod, RpcRequest, RpcResponse, RpcService, SimpleRpcService};

struct Echo;
// implementar RpcMethod<Request=Vec<u8>, Response=Vec<u8>> para Echo ...

let mut service = SimpleRpcService::new("demo");
service.register(Echo);
```

Métodos:

- `new(name: impl Into<String>) -> Self` — cria um novo serviço simples

---

### RpcServiceBuilder

Builder de serviço RPC que registra métodos e constrói um `SimpleRpcService`.

```rust
use gerax_rpc::{RpcServiceBuilder, Echo};

let service = RpcServiceBuilder::new("demo")
    .method(Echo)
    .build();
```

Métodos:

- `new(name: impl Into<String>) -> Self` — cria um novo builder
- `method<M: RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + 'static>(self, method: M) -> Self` — registra um método
- `build(self) -> SimpleRpcService` — constrói o serviço

---

### RpcTransport

Trait que abstrai a camada de transporte (TCP, QUIC, HTTP/2, etc.) independente de protocolo.

```rust
pub trait RpcTransport: Send + Sync + 'static {
    type Connection: Send + Sync + 'static;

    async fn connect(&self) -> Result<Self::Connection, RpcError>;
    async fn send(&self, conn: &mut Self::Connection, request: RpcRequest<Vec<u8>>) -> Result<RpcResponse<Vec<u8>>, RpcError>;
    async fn close(&self, conn: Self::Connection) -> Result<(), RpcError>;
}
```

Tipo associado:

- `Connection: Send + Sync + 'static` — tipo da conexão

Métodos obrigatórios:

- `connect(&self) -> Result<Self::Connection, RpcError>` — estabelece uma conexão
- `send(&self, conn: &mut Self::Connection, request: RpcRequest<Vec<u8>>) -> Result<RpcResponse<Vec<u8>>, RpcError>` — envia requisição e recebe resposta
- `close(&self, conn: Self::Connection) -> Result<(), RpcError>` — fecha a conexão

---

### RpcClient

Trait que abstrai a chamada RPC do lado cliente, independente de protocolo.

```rust
pub trait RpcClient: Send + Sync + 'static {
    type Request;
    type Response;

    async fn call(&self, method: &str, request: RpcRequest<Self::Request>) -> RpcResponse<Self::Response>;
}
```

Tipos associados:

- `Request` — tipo da requisição serializada
- `Response` — tipo da resposta deserializada

Métodos obrigatórios:

- `call(&self, method: &str, request: RpcRequest<Self::Request>) -> RpcResponse<Self::Response>` — invoca um método remotamente

---

### TransportRpcClient\<T\>

Cliente RPC baseado em transporte. Implementa `RpcClient`.

```rust
use gerax_rpc::TransportRpcClient;

let client = TransportRpcClient::new(meu_transporte);
```

Métodos:

- `new(transport: T) -> Self` — cria um novo cliente com o transporte fornecido

---

### RpcServer

Trait que abstrai a execução de um serviço RPC, independente de protocolo.

```rust
pub trait RpcServer: Send + Sync + 'static {
    async fn serve(&self, addr: &str) -> Result<(), RpcError>;
    async fn shutdown(&self) -> Result<(), RpcError>;
}
```

Métodos obrigatórios:

- `serve(&self, addr: &str) -> Result<(), RpcError>` — inicia o servidor
- `shutdown(&self) -> Result<(), RpcError>` — para o servidor

---

### TransportRpcServer\<T, S\>

Servidor RPC baseado em transporte. Implementa `RpcServer`.

```rust
use gerax_rpc::{TransportRpcServer, SimpleRpcService};

let server = TransportRpcServer::new(meu_transporte, SimpleRpcService::new("my-service"));
```

Métodos:

- `new(transport: T, service: S) -> Self` — cria um novo servidor com transporte e serviço

---

### RpcStream\<T\>

Trait que abstrai streams de dados bidirecionais, independente de protocolo.

```rust
pub trait RpcStream<T>: Send + Sync + 'static {
    async fn recv(&mut self) -> Option<Result<T, RpcError>>;
    async fn send(&mut self, item: T) -> Result<(), RpcError>;
    async fn close(self) -> Result<(), RpcError>;
}
```

Métodos obrigatórios:

- `recv(&mut self) -> Option<Result<T, RpcError>>` — recebe o próximo item
- `send(&mut self, item: T) -> Result<(), RpcError>` — envia um item
- `close(self) -> Result<(), RpcError>` — fecha o stream

---

### RpcResponseStream\<T\>

Alias para stream de resposta do lado servidor.

```rust
pub type RpcResponseStream<T> = Box<dyn Stream<Item = Result<T, RpcError>> + Send + Unpin>;
```

---

## Erros

Todas as operações do crate utilizam `Result<T, RpcError>`.

Variações de `RpcError`:

- `Transport(String)` — erro de transporte ou conexão
- `Serialization(String)` — erro de serialização/deserialização
- `NotFound(String)` — recurso não encontrado
- `Internal(String)` — erro interno do servidor
- `InvalidRequest(String)` — requisição inválida
- `Timeout(String)` — timeout na requisição
- `Unauthenticated(String)` — erro de autenticação
- `PermissionDenied(String)` — erro de autorização
- `ResourceExhausted(String)` — recurso esgotado (rate limit, quota)
- `Unimplemented(String)` — funcionalidade não implementada

---

## Exemplos

### Retaguarda Síncrona

```rust
use gerax_rpc::{
    RpcContextBuilder, RpcExtensions, RpcMetadataBuilder, RpcRequestBuilder,
    RpcResponse, RpcResponseBuilder, RpcStatus,
};

fn main() {
    let metadata = RpcMetadataBuilder::new()
        .insert("authorization", "Bearer token")
        .insert("trace-id", "abc123")
        .build();

    let context = RpcContextBuilder::new()
        .metadata(metadata.clone())
        .trace_id("abc123")
        .build();

    let request = RpcRequestBuilder::new("ping".to_string())
        .metadata(metadata)
        .context(context)
        .build();

    println!("payload: {}", request.payload);

    let ok: RpcResponse<String> = RpcResponse::success("pong".to_string());
    assert!(!ok.is_error());

    let error = RpcResponseBuilder::<String>::new()
        .status(RpcStatus::NotFound)
        .message("recurso não encontrado")
        .build();
    assert!(error.is_error());
}
```

### Serviço RPC

```rust
use async_trait::async_trait;
use futures::executor::block_on;
use gerax_rpc::{RpcMethod, RpcRequest, RpcResponse, RpcService, RpcServiceBuilder};

struct Echo;

#[async_trait]
impl RpcMethod for Echo {
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str { "echo" }

    async fn call(&self, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        RpcResponse::success(request.payload)
    }
}

fn main() {
    let service = RpcServiceBuilder::new("demo")
        .method(Echo)
        .build();

    let resp = block_on(service.call("echo", RpcRequest::new(b"hello".to_vec())));
    println!("{:?}", resp.payload);
}
```

### Cliente RPC

```rust
use async_trait::async_trait;
use futures::executor::block_on;
use gerax_rpc::{RpcClient, RpcError, RpcRequest, RpcResponse, RpcTransport, TransportRpcClient};

#[derive(Clone)]
struct EchoTransport;

#[async_trait]
impl RpcTransport for EchoTransport {
    type Connection = ();

    async fn connect(&self) -> Result<(), RpcError> { Ok(()) }

    async fn send(
        &self,
        _conn: &mut (),
        request: RpcRequest<Vec<u8>>,
    ) -> Result<RpcResponse<Vec<u8>>, RpcError> {
        Ok(RpcResponse::success(request.payload))
    }

    async fn close(&self, _conn: ()) -> Result<(), RpcError> { Ok(()) }
}

fn main() {
    let client = TransportRpcClient::new(EchoTransport);
    let resp = block_on(client.call("echo", RpcRequest::new(b"ping".to_vec())));
    println!("{:?}", resp.payload);
}
```
