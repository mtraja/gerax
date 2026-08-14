# plano-funcional.md — Torna `gerax-grpc` funcional

## Problemas atuais

1. `GrpcServer::serve()` é stub (não inicia servidor tonic real).
2. `GrpcClient` usa `NoOpInvoker` (não faz roundtrip real).
3. `GrpcClient::connect()` vaza `tonic::transport::Channel` na API pública.

## Decisão de arquitetura

Manter a abstração `gerax-rpc::RpcService` e adicionar uma **camada de transporte
tonic real** sem gerar stubs por entidade.

Estratégia: **serviço genérico fixo compilado com tonic-build**.

- Definir um schema Protobuf fixo em `proto/gerax_rpc.proto` com um serviço
  `GeraxRpc` e um único método `Call`.
- `tonic-build` (já em build-dependencies) compila esse proto em `build.rs`.
- Implementar `GeraxRpcServer` delegando para `dyn RpcService`.
- Cliente usa `GeraxRpcClient` + `tonic::transport::Channel` internamente.
- `GrpcClient` gerencia conexões internamente; não expõe tipos tonic.

Schema fixo (`proto/gerax_rpc.proto`):

```proto
syntax = "proto3";
package gerax;

service GeraxRpc {
  rpc Call(RpcRequest) returns (RpcResponse);
}

message RpcRequest {
  string method = 1;
  bytes payload = 2;
  map<string, string> metadata = 3;
}

message RpcResponse {
  bytes payload = 1;
  int32 status = 2;
  string message = 3;
}
```

## Passos de implementação

### Passo 1 — `proto/gerax_rpc.proto` + `build.rs`

- Criar `crates/gerax-grpc/proto/gerax_rpc.proto`.
- Atualizar/criar `build.rs` para invocar `tonic-build::compile()` apenas
  para esse arquivo, gerando código em `OUT_DIR`.
- Re-exportar tipos gerados em `src/proto/mod.rs` (feature-gated para `tonic`).

### Passo 2 — Adaptador tonic server (`src/transport/tonic_server_adapter.rs`)

Implementar a trait gerada `gerax_rpc_server::GeraxRpc` por meio de um
adaptador genérico:

```rust
struct GeraxRpcServerAdapter<S> {
    service: Arc<S>,
}

#[async_trait]
impl<S: RpcService + Send + Sync + 'static> GeraxRpc for GeraxRpcServerAdapter<S> {
    async fn call(
        &self,
        request: Request<RpcRequestProto>,
    ) -> Result<Response<RpcResponseProto>, tonic::Status> {
        let req = request.into_inner();
        let mut meta = RpcMetadata::new();
        for (k, v) in req.metadata { meta.insert(k, v); }
        let rpc_req = RpcRequest::new(req.payload).with_metadata(meta);
        let response = self.service.call(&req.method, rpc_req).await;
        // mapear RpcResponse -> RpcResponseProto
    }
}
```

### Passo 3 — `GrpcServer::serve()` funcional (`src/server.rs`)

Remover stub e substituir por:

```rust
pub async fn serve(self) -> Result<(), GrpcError> {
    let addr = self.addr.parse::<SocketAddr>()?;
    let adapter = GeraxRpcServerAdapter {
        service: Arc::new(self.service),
    };
    tonic::transport::Server::builder()
        .add_service(GeraxRpcServer::new(adapter))
        .serve(addr)
        .await
        .map_err(|e| GrpcError::RpcError(e.to_string()))
}
```

### Passo 4 — Adaptador tonic client (`src/transport/tonic_client_adapter.rs`)

Substituir `NoOpInvoker` por `TonicRpcInvoker`:

```rust
pub struct TonicRpcInvoker {
    client: GeraxRpcClient<tonic::transport::Channel>,
}

#[async_trait]
impl GrpcInvoker for TonicRpcInvoker {
    async fn invoke(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError> {
        let response = self.client
            .call(Request::new(RpcRequestProto { method, payload, metadata: HashMap::new() }))
            .await
            .map_err(|e| GrpcError::RpcError(e.to_string()))?;
        // mapear resposta
    }
}
```

### Passo 5 — Remover vazamento de tonic da API pública (`src/client.rs`)

Redesenhar `GrpcClient` para gerenciar conexão internamente:

```rust
pub struct GrpcClient<T, C = JsonCodec> {
    endpoint: String,
    codec: Arc<C>,
    inner: Arc<Mutex<Option<tonic::transport::Channel>>>,
    _marker: PhantomData<T>,
}

impl<T, C> GrpcClient<T, C> {
    pub fn new(endpoint: impl Into<String>, codec: C) -> Self { ... }

    // Remove connect() da API pública ou retorna Result<()>
    pub async fn connect(&self) -> Result<(), GrpcError> { ... }

    async fn channel(&self) -> Result<tonic::transport::Channel, GrpcError> { ... }
}
```

- `connect()` deixa de retornar `tonic::transport::Channel`.
- `send_request()` usa `channel()` internamente para criar o invoker.
- Atualizar testes e exemplos que usam `connect()`.

### Passo 6 — Feature `tonic` em `Cargo.toml`

Adicionar feature para opcionalizar a integração real:

```toml
[features]
default = ["tonic"]
tonic = ["dep:tonic", "dep:prost", "dep:tonic-build"]
```

- O código atual (stub) fica disponível sem a feature.
- Com `--features tonic`, habilita servidor/cliente reais.

### Passo 7 — Testes de roundtrip (`tests/grpc_roundtrip.rs`)

- `GrpcServer` com repositório em memória em background task.
- `GrpcClient` conecta, faz CRUD e valida respostas.
- Garantir que `DbError::NotFound` mapeia para gRPC status correto.

### Passo 8 — Atualizar exemplos e README

- `examples/crud.rs` demonstra roundtrip real.
- `examples/proto.rs` continua funcional.
- `README.md` documenta feature `tonic`.

## Ordem sugerida

1. Passo 1 (proto + build.rs)
2. Passo 2 (adaptador server)
3. Passo 3 (serve funcional)
4. Passo 4 (adaptador client)
5. Passo 5 (remover vazamento)
6. Passo 6 (feature)
7. Passo 7 (testes)
8. Passo 8 (docs)

## Aceitação

- `GrpcServer::serve()` inicia servidor tonic real.
- `GrpcClient::find_by_id()` / `insert()` / etc. fazem roundtrip.
- Nenhum tipo tonic aparece nas assinaturas públicas de `GrpcClient`.
- `cargo test -p gerax-grpc --features tonic` passa.
