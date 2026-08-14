# plano.md — Implementação da crate `gerax-grpc`

## Visão Geral

Implementar o suporte a gRPC no ecossistema Gerax, integrando as crates
`gerax-core`, `gerax-db`, `gerax-codec`, `gerax-rpc` e `gerax-http`.

A arquitetura segue a separação de responsabilidades já estabelecida:

```
gerax-grpc
├── proto/          Geração de schema Protobuf a partir de entidades
├── transport/      Implementação de RpcTransport usando tonic
├── server/         GrpcServer<T> + integração com Repository<T>
├── client/         GrpcClient<T> com CRUD tipado
├── method/         Métodos RPC genéricos (CRUD)
├── error/          GrpcError (mapeamento de tonic/DbError/RpcError)
└── builder.rs      Builders fluentes para server/client/service
```

## Dependências e pré-requisitos

### Compatibilidade `prost` / `tonic`

O workspace está unificado em `prost = "0.14"` e `tonic = "0.14"`, removendo o
conflito anterior entre `tonic 0.12` + `prost 0.13` e `gerax-codec` (`prost 0.14`).

### Dependências da crate

```toml
[dependencies]
gerax-core.workspace = true
gerax-db.workspace = true
gerax-http.workspace = true
gerax-codec.workspace = true
gerax-rpc.workspace = true

async-trait.workspace = true
serde.workspace = true
thiserror.workspace = true
tonic.workspace = true
prost.workspace = true
prost-types.workspace = true

[build-dependencies]
tonic-build.workspace = true
```

## Estrutura alvo

```
crates/gerax-grpc/
├── Cargo.toml
├── README.md
├── plano.md
└── src/
    ├── lib.rs
    ├── builder.rs
    ├── error/
    │   └── mod.rs
    ├── proto/
    │   ├── mod.rs
    │   └── builder.rs
    ├── transport/
    │   ├── mod.rs
    │   ├── client_transport.rs
    │   └── server_transport.rs
    ├── server/
    │   ├── mod.rs
    │   ├── server.rs
    │   └── service.rs
    ├── client/
    │   ├── mod.rs
    │   └── client.rs
    └── method/
        ├── mod.rs
        └── crud.rs
```

## Tarefas

### Fase 1 — Fundação

#### 1.1 Corrigir `Cargo.toml`
- `tonic` e `prost` já foram atualizados no workspace para `0.14`, compatíveis entre si.
- Adicionar `gerax-rpc`, `gerax-codec`, `prost-types` e `tonic-build`.
- `gerax-http` já está presente; reavaliar necessidade na Fase 6.

#### 1.2 Reestruturar módulos
- Criar `src/transport/mod.rs`, `src/method/mod.rs`, `src/error/mod.rs`.
- Mover `src/client/client.rs` e `src/server/server.rs` para estrutura plana
  (`src/client.rs`, `src/server.rs`) ou manter subdiretórios com `mod.rs`.
  Seguir o padrão já adotado em `gerax-rpc` (arquivos planos).
- Atualizar `src/lib.rs` com `pub mod` e re-exports.

#### 1.3 Definir tipos fundamentais
- `GrpcError` em `error/mod.rs` com variantes:
  - `RpcError(String)`
  - `SerializationError(String)`
  - `NotFound(String)`
- `ProtoSchema` em `proto/mod.rs` — wrapper para o schema Protobuf gerado
  (arquivo `.proto` como `String` ou estruturado).

---

### Fase 2 — Geração de schema Protobuf

#### 2.1 Mapeamento `Entity` → Protobuf
- Mapear tipos Rust comuns (`String`, `i32`, `i64`, `f64`, `bool`, `Vec<T>`)
  para tipos Protobuf (`string`, `int32`, `int64`, `double`, `bool`, `repeated`).
- Definir convenção de nomes: `Entity::collection_name()` vira nome do tipo
  Protobuf em `PascalCase`.
- Ignorar campos `id` e `set_id` do trait `Entity` ou mapeá-los como
  `optional string id = 1;`.

#### 2.2 Implementar `ProtoBuilder`
- `ProtoBuilder::new() -> Self`
- `ProtoBuilder::entity<T>() -> Self` — registra entidade usando
  `T::collection_name()` e inspeciona seus campos via reflection básico
  (espera-se que `Entity` derive `Serialize`/`Deserialize`, então é possível
  inspecionar com `serde` ou exigir que o usuário forneça metadados).
- `ProtoBuilder::build() -> ProtoSchema` — gera o conteúdo `.proto` completo,
  incluindo:
  - `syntax = "proto3";`
  - `package gerax;`
  - Mensagens para cada entidade
  - Serviço `EntityService` com RPCs CRUD:
    - `Create`, `Read`, `Update`, `Delete`, `List`

#### 2.3 Testes de schema
- Testar que `ProtoBuilder` gera proto válido para entidades simples.
- Verificar nomes de pacotes, tipos e serviços.

---

### Fase 3 — Camada de transporte gRPC (ponte com `gerax-rpc`)

#### 3.1 `TonicClientTransport`
- Implementar `gerax_rpc::RpcTransport` para o lado cliente.
- `Connection = tonic::transport::Channel`
- `connect()` — cria canal tonic.
- `send()` — serializa `RpcRequest<Vec<u8>>`, invoca método gRPC via stub
  genérico, desserializa resposta.
- `close()` — encerra canal.

#### 3.2 `TonicServerTransport`
- Implementar `gerax_rpc::RpcTransport` para o lado servidor.
- `Connection` — wrapper ao redor do stream de requisições tonic.
- `connect()` — aceita conexão incoming.
- `send()` — recebe requisição gRPC, desserializa, retorna resposta.
- `close()` — finaliza stream.

#### 3.3 Codec no transporte
- Usar `gerax_codec::ProtobufCodec` (ou `gerax_codec::Codec` genérico) para
  codificar/decodificar payloads no transporte.
- Atenção: `ProtobufCodec` exige `T: prost::Message + Default`.
- Para transporte genérico `Vec<u8>`, o codec opera sobre tipos intermediários
  definidos no schema gerado na Fase 2.

---

### Fase 4 — Servidor gRPC

#### 4.1 `GrpcService<T>` (adaptador `RpcService`)
- Implementar `gerax_rpc::RpcService` para expor CRUD de uma entidade `T`.
- Registrar métodos:
  - `create_<entity>` — insert
  - `read_<entity>` — find_by_id
  - `update_<entity>` — update
  - `delete_<entity>` — delete
  - `list_<entity>` — find_all
- Cada método desserializa o request, chama `Repository<T>` e serializa a resposta.

#### 4.2 `GrpcServer<T>`
- Estrutura:
  ```rust
  pub struct GrpcServer<T, S = GrpcService<T>> {
      service: S,
      repo: Arc<dyn Repository<T>>,
      addr: String,
  }
  ```
- `GrpcServer::new() -> Self`
- `serve(self, addr: &str) -> Result<(), GrpcError>`
  - Inicia `tonic::transport::Server::builder()`
  - Adiciona serviço gerado pelo builder de proto
  - Bloqueia até shutdown ou erro.

#### 4.3 Integração com `gerax-db::Repository<T>`
- `GrpcServer` recebe ou constrói um repositório via `RepositoryBuilder`.
- Mapear `DbError` para `GrpcError` / `RpcStatus`.

---

### Fase 5 — Cliente gRPC

#### 5.1 `GrpcClient<T>`
- Estrutura:
  ```rust
  pub struct GrpcClient<T> {
      inner: GrpcClientInner, // stub tonic tipado
  }
  ```
- `GrpcClient::new() -> Self`
- `connect(&self, addr: &str) -> Result<(), GrpcError>`
  - Cria `tonic::transport::Channel::builder().connect(addr).await?`
- `call(&self, request: T) -> Result<T, GrpcError>`
  - Serializa request usando codec apropriado
  - Invoca método gRPC
  - Desserializa resposta

#### 5.2 Métodos de CRUD (opcional, por conveniência)
- `find_by_id(&self, id: &str) -> Result<Option<T>, GrpcError>`
- `find_all(&self) -> Result<Vec<T>, GrpcError>`
- `insert(&self, entity: T) -> Result<T, GrpcError>`
- `update(&self, entity: T) -> Result<(), GrpcError>`
- `delete(&self, id: &str) -> Result<(), GrpcError>`

Esses métodos são atalhos sobre `call` ou implementações diretas usando o stub tonic.

---

### Fase 6 — Integração com `gerax-http`

#### 6.1 Handler HTTP para gRPC
- Fornecer `GrpcHandler` que implementa `gerax_http::Handler`.
- Permitir que `gerax-http` exponha endpoints gRPC (ex: gRPC-Web ou proxy).
- Ou fornecer um builder que registra o servidor gRPC como parte do stack HTTP.

#### 6.2 Configuração
- Ler configuração de endereço/porta de `gerax-config` quando aplicável.

---

### Fase 7 — Testes e exemplos

#### 7.1 Testes de integração (`tests/`)
- Testar `ProtoBuilder` gerando schema válido.
- Testar `GrpcServer` + `GrpcClient` com repositório em memória (MongoDB/SQLite fake).
- Testar mapeamento de erros (`DbError` → `GrpcError`).
- Testar serialização/deserialização Protobuf de entidades.

#### 7.2 Exemplos (`examples/`)
- `examples/crud.rs` — definir entidade, gerar proto, subir servidor, chamar cliente.
- `examples/proto.rs` — usar `ProtoBuilder` e inspecionar schema gerado.

---

### Fase 8 — Documentação

#### 8.1 `README.md`
- Seguir formato `create-doc` (padrão `gerax-config`).
- Documentar:
  - `ProtoBuilder`
  - `ProtoSchema`
  - `GrpcServer<T>`
  - `GrpcClient<T>`
  - `GrpcError`
  - `GrpcService<T>` (se exposto)
  - Handlers de integração com `gerax-http`
- Incluir exemplos funcionais.

---

## Ordem de implementação sugerida

1. **Fase 1.1 e 1.2** — corrigir dependências e estrutura.
2. **Fase 2** — `ProtoBuilder` e schema generation (pode ser desenvolvido isoladamente).
3. **Fase 3** — transporte tonic (`RpcTransport`).
4. **Fase 4** — servidor e integração com `Repository<T>`.
5. **Fase 5** — cliente e CRUD.
6. **Fase 6** — integração com `gerax-http`.
7. **Fase 7 e 8** — testes, exemplos e documentação.

## Critérios de aceitação

- `cargo fmt --check -p gerax-grpc`
- `cargo clippy -p gerax-grpc --all-targets --all-features -D warnings`
- `cargo test -p gerax-grpc`
- `cargo doc --workspace --all-features`
- `ProtoBuilder::build()` gera schema Protobuf válido para entidades de teste.
- `GrpcServer::serve()` inicia servidor gRPC funcional com CRUD.
- `GrpcClient::connect()` + `call()` roundtrip com sucesso.
- Integração com `gerax-db::Repository<T>` retorna `GrpcError::NotFound` quando aplicável.
- Sem vazamento de tipos internos de `tonic` na API pública.
- Pronto para ser usado pela fachada `gerax` e exemplos.
