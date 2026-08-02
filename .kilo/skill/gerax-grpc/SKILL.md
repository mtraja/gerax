---
name: gerax-grpc
description: Implementa a crate gerax-grpc seguindo o spec README.md
---

# Skill: gerax-grpc

Use quando precisar implementar ou modificar o suporte a gRPC na crate `gerax-grpc`.

## Especificação

Leia `crates/gerax-grpc/README.md` antes de implementar.

## Contratos obrigatórios

Implemente os seguintes componentes na crate `gerax-grpc`:

### `ProtoBuilder`

- `new() -> Self` — cria um builder vazio.
- `entity<T: Entity + Send + Sync + 'static>(mut self) -> Self` — registra uma entidade `gerax-core::Entity` no schema Protobuf.
- `build(self) -> ProtoSchema` — gera o schema Protobuf a partir das entidades registradas.

### `GrpcServer<T>`

- `new() -> Self` — cria um servidor gRPC para a entidade `T`.
- `serve(self, addr: &str) -> Result<(), GrpcError>` — inicia o servidor gRPC no endereço especificado e bloqueia até encerramento ou erro.

### `GrpcClient<T>`

- `new() -> Self` — cria um cliente gRPC para a entidade `T`.
- `connect(&self, addr: &str) -> Result<(), GrpcError>` — conecta ao servidor gRPC no endereço especificado.
- `call(&self, request: T) -> Result<T, GrpcError>` — envia uma requisição e retorna a resposta.

### `GrpcError`

- `RpcError(String)` — erro retornado pelo servidor ou transporte gRPC.
- `SerializationError(String)` — erro de serialização/deserialização de mensagens.
- `NotFound(String)` — recurso não encontrado no servidor.

## Regras

- Depende de `gerax-core`, `gerax-http`, `gerax-db`, `async-trait`, `serde`, `thiserror`.
- Usa `tonic` como runtime gRPC e `prost` para serialização Protobuf.
- Todos os métodos são async e usam `async-trait` quando aplicável.
- Erros usam `thiserror` e convertem para `gerax-core::Error` quando relevante.
- `ProtoBuilder` usa `gerax_core::Entity::collection_name()` para nomear tipos/mensagens Protobuf.
- O servidor deve integrar com `gerax-db::Repository<T>` para resolver dados das entidades.
- O cliente deve integrar com `gerax-http` para expor o endpoint gRPC.
- Não vaze tipos internos do `tonic` na API pública.
- Parâmetros de configuração devem vir de `gerax-config` quando aplicável.

## Casos de erro

- RpcError
- SerializationError
- NotFound
- ConnectionError (do gerax-db)
- NotFoundError (do gerax-db)

## Testes esperados

- Deve criar uma instância de `ProtoBuilder`.
- Deve registrar uma entidade no schema via `entity::<T>()`.
- Deve gerar um schema Protobuf válido via `build()`.
- Deve criar uma instância de `GrpcServer`.
- Deve criar uma instância de `GrpcClient`.
- Deve retornar `GrpcError::RpcError` para erros de transporte.
- Deve retornar `GrpcError::SerializationError` para falhas de serialização.
- Deve retornar `GrpcError::NotFound` quando recurso não existir.
- Deve integrar com `gerax-db::Repository<T>` para buscar dados de entidades.