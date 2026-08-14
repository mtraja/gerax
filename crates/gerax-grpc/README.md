# gerax-grpc

Crate de suporte a gRPC no framework Gerax.

## Dependências

```toml
gerax-grpc = { path = "../crates/gerax-grpc" }
```

## Features

Nenhuma feature customizada. O build com `tonic-prost-build` gera os stubs Protobuf e tonic.

## Visão Geral

`gerax-grpc` integra entidades `gerax-core` com o ecossistema gRPC. Gera schemas
Protobuf automaticamente, expõe serviços CRUD genéricos e fornece transporte
cliente/servidor baseado em `tonic`.

```
ProtoBuilder -> ProtoSchema
Entity + Repository -> GrpcService -> GrpcServer -> serve()
Entity + Repository -> GrpcService -> GrpcHandler -> Router (gerax-http)
Endpoint + Entity -> GrpcClient -> call() / find_by_id() / ...
```

## API

### ProtoBuilder

Builder para geração de schema Protobuf a partir de entidades Gerax.

```rust
let schema = ProtoBuilder::new()
    .entity::<User>()
    .field("name", RustType::String)
    .field("email", RustType::String)
    .build();
```

Métodos:

- `new() -> Self` — cria um novo builder vazio.
- `entity<T: Entity + Send + Sync + 'static>(self) -> Self` — registra uma entidade no schema.
- `field(self, name: impl Into<String>, rust_type: RustType) -> Self` — adiciona um campo à última entidade registrada.
- `build(self) -> ProtoSchema` — gera o schema Protobuf completo.

---

### ProtoSchema

Schema Protobuf gerado.

```rust
println!("package: {:?}", schema.package_name());
println!("services: {:?}", schema.services().collect::<Vec<_>>());
println!("{}", schema.content);
```

Métodos:

- `new(file_name: impl Into<String>, content: impl Into<String>) -> Self`
- `package_name(&self) -> Option<&str>` — retorna o nome do pacote declarado no schema.
- `services(&self) -> impl Iterator<Item = &str>` — itera sobre os nomes de serviços declarados.
- `into_value(self) -> Value` — consome o schema e retorna o `Value` JSON interno.
- `deserialize<T: DeserializeOwned>(&self) -> ConfigResult<T>` — desserializa para uma estrutura Rust.

Campos públicos:

- `file_name: String`
- `content: String`

---

### RustType

Mapeamento de tipos Rust para tipos Protobuf.

```rust
.field("items", RustType::Vec(Box::new(RustType::String)))
```

Variantes:

- `String`
- `I32`
- `I64`
- `F32`
- `F64`
- `Bool`
- `Vec(Box<RustType>)`

Métodos:

- `to_proto_type(&self) -> String` — retorna a representação Protobuf do tipo.

---

### GrpcServer<T, C>

Servidor gRPC genérico.

```rust
let server = GrpcServer::new(service, "0.0.0.0:50051");
server.serve().await?;
```

Métodos:

- `new(service: GrpcService<T, C>, addr: impl Into<String>) -> Self` — cria um novo servidor com o serviço e endereço fornecidos.
- `from_builder(name: impl Into<String>, builder: impl RepositoryBuilder<T>, codec: C, addr: impl Into<String>) -> Result<Self, GrpcError>` — cria um servidor a partir de um `RepositoryBuilder`.
- `serve(self) -> Result<(), GrpcError>` — inicia o servidor no endereço especificado.
- `from_config(service: GrpcService<T, C>, config: &GrpcConfig) -> Self` — cria um servidor a partir de uma configuração `GrpcConfig`.

---

### GrpcClient<T, C>

Cliente gRPC genérico.

```rust
let client = GrpcClient::<User>::new("http://127.0.0.1:50051", JsonCodec);
let user = client.find_by_id("1").await?;
```

Métodos:

- `new(endpoint: impl Into<String>, codec: C) -> Self` — cria um novo cliente gRPC.
- `connect(&self) -> Result<(), GrpcError>` — conecta ao servidor gRPC.
- `call(&self, method: &str, request: T) -> Result<T, GrpcError>` — invoca um método gRPC remotamente.
- `call_entity(&self, method: &str, request: T) -> Result<T, GrpcError>` — invoca um método gRPC com a entidade `T`.
- `find_by_id(&self, id: &str) -> Result<Option<T>, GrpcError>` — atalho para `read_<collection>`.
- `find_all(&self) -> Result<Vec<T>, GrpcError>` — atalho para `list_<collection>`.
- `insert(&self, entity: T) -> Result<T, GrpcError>` — atalho para `create_<collection>`.
- `update(&self, entity: T) -> Result<(), GrpcError>` — atalho para `update_<collection>`.
- `delete(&self, id: &str) -> Result<(), GrpcError>` — atalho para `delete_<collection>`.

---

### GrpcService<T, C>

Serviço gRPC genérico que expõe CRUD de uma entidade.

Implementa `gerax_rpc::RpcService`. Registra automaticamente os métodos:
`create_<entity>`, `read_<entity>`, `update_<entity>`, `delete_<entity>`,
`list_<entity>`.

```rust
let service = GrpcService::new("users", repo, JsonCodec);
```

Métodos:

- `new(name: impl Into<String>, repo: Arc<dyn Repository<T>>, codec: C) -> Self` — cria um novo serviço gRPC.
- `from_builder(name: impl Into<String>, builder: impl RepositoryBuilder<T>, codec: C) -> Result<Self, GrpcError>` — cria um serviço a partir de um `RepositoryBuilder`.

Métodos de `RpcService`:

- `name(&self) -> &str` — nome do serviço.
- `register<M>(&mut self, method: M)` — registra um método no serviço.
- `call(&self, method: &str, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>>` — invoca um método pelo nome.

---

### GrpcHandler<T, C>

Handler HTTP que expõe um `GrpcService` como proxy RPC via `gerax-http`.

```rust
let handler = GrpcHandler::new(service);
let router = Router::new().post("/grpc", handler);
```

Métodos:

- `new(service: GrpcService<T, C>) -> Self` — cria um novo handler a partir de um `GrpcService`.
- `from_service(service: Arc<GrpcService<T, C>>) -> Self` — cria um handler a partir de um `GrpcService` compartilhado.
- `register<State>(self, router: Router<State>, path: impl Into<String>) -> Router<State>` — registra o handler em um `Router` no path informado (método HTTP POST).

Implementa `gerax_http::Handler<State>`.

---

### GrpcConfig

Configuração de bind do servidor gRPC.

```rust
#[derive(serde::Deserialize)]
struct AppConfig {
    grpc: GrpcConfig,
}

let config: AppConfig = gerax_config::Config::builder()
    .toml("config.toml")
    .build()?;
let addr = config.grpc.socket_addr();
```

Campos públicos:

- `host: String`
- `port: u16`

Métodos:

- `socket_addr(&self) -> String` — retorna o endereço de socket completo (`host:port`).
- `from_document(doc: &ConfigDocument) -> ConfigResult<Self>` — desserializa a configuração a partir de um documento `gerax-config`.
- `from_builder(builder: ConfigBuilder) -> ConfigResult<Self>` — carrega a configuração a partir de um builder `gerax-config`.

---

### GrpcError

Erros de execução gRPC.

```rust
match err {
    GrpcError::NotFound(id) => ...,
    GrpcError::SerializationError(msg) => ...,
    GrpcError::RpcError(msg) => ...,
    GrpcError::InvalidRequest(msg) => ...,
}
```

Variações:

- `RpcError(String)` — erro retornado pelo servidor ou transporte gRPC.
- `InvalidRequest(String)` — requisição inválida (ex: payload malformado).
- `SerializationError(String)` — erro de serialização/deserialização de mensagens.
- `NotFound(String)` — recurso não encontrado no servidor.

---

### GrpcInvoker

Trait para invocação de métodos gRPC no lado cliente.

```rust
#[async_trait]
impl GrpcInvoker for MyInvoker {
    async fn invoke(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError> {
        ...
    }
}
```

Métodos:

- `invoke(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError>` — invoca um método gRPC pelo nome e retorna o payload da resposta.

---

### GrpcCodecInvoker<C, T>

Trait para invocação de métodos gRPC com suporte a codec.

```rust
let result = invoker.invoke_typed("create_user", entity, &JsonCodec).await?;
```

Métodos:

- `invoke_typed(&self, method: &str, payload: T, codec: &C) -> Result<T, GrpcError>` — invoca um método gRPC com payload tipado.

---

### TonicClientTransport<I>

Transporte gRPC cliente baseado em `tonic`.

```rust
let transport = TonicClientTransport::new("http://127.0.0.1:50051", invoker);
```

Implementa `gerax_rpc::RpcTransport`.

Campos públicos:

- `endpoint: String`
- `invoker: I`

Métodos:

- `new(endpoint: impl Into<String>, invoker: I) -> Self` — cria um novo transporte cliente.

---

### GrpcRequestHandler

Trait para processamento de requisições gRPC no lado servidor.

```rust
#[async_trait]
impl GrpcRequestHandler for MyHandler {
    async fn handle(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError> {
        ...
    }
}
```

Métodos:

- `handle(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError>` — processa uma requisição gRPC e retorna o payload da resposta.

---

### GrpcCodecHandler<C, T>

Trait para processamento de requisições gRPC com suporte a codec.

```rust
let result = handler.handle_typed("create_user", entity, &JsonCodec).await?;
```

Métodos:

- `handle_typed(&self, method: &str, payload: T, codec: &C) -> Result<T, GrpcError>` — processa uma requisição gRPC com payload tipado.

---

### TonicServerConnection

Handle de conexão do transporte servidor.

```rust
let conn = TonicServerConnection::new();
```

Métodos:

- `new() -> Self` — cria uma nova conexão servidor vazia.

---

### TonicServerTransport<H>

Transporte gRPC servidor baseado em `tonic`.

```rust
let transport = TonicServerTransport::new(handler);
```

Implementa `gerax_rpc::RpcTransport`.

Campos públicos:

- `handler: H`

Métodos:

- `new(handler: H) -> Self` — cria um novo transporte servidor.

---

## Exemplos

```rust
use gerax_grpc::{GrpcConfig, GrpcServer, ProtoBuilder, RustType};
use gerax_codec::JsonCodec;
use gerax_core::Entity;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct User { id: String, name: String }

impl Entity for User {
    fn collection_name() -> &'static str { "users" }
    fn id(&self) -> Option<String> { Some(self.id.clone()) }
    fn set_id(&mut self, id: String) { self.id = id; }
}

let schema = ProtoBuilder::new()
    .entity::<User>()
    .field("name", RustType::String)
    .build();

println!("package: {:?}", schema.package_name());
println!("services: {:?}", schema.services().collect::<Vec<_>>());

let config = GrpcConfig::default();
println!("bind addr: {}", config.socket_addr());

let client = GrpcClient::<User>::new("http://127.0.0.1:50051", JsonCodec);
client.connect().await.expect("connect");
```
