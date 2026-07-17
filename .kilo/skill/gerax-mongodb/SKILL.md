---
name: gerax-mongodb
description: Implementa o Repository para MongoDB na crate gerax-mongodb seguindo o spec SPEC.md
---

# Skill: gerax-mongodb

Use quando precisar implementar ou modificar o repositório MongoDB na crate `gerax-mongodb`.

## Especificação

Leia `crates/gerax-mongodb/SPEC.md` antes de implementar.

## Contratos obrigatórios

Implemente `gerax-db::Repository<T>` para **MongoDB** via driver oficial `mongodb`:

- `find_by_id(id: &str) -> Result<Option<T>, DbError>`
- `find_all() -> Result<Vec<T>, DbError>`
- `insert(entity: T) -> Result<T, DbError>`
- `update(entity: T) -> Result<(), DbError>`
- `delete(id: &str) -> Result<(), DbError>`

Implemente também `gerax-db::Connection`:

- `connect() -> Result<Self, DbError>`
- `ping(&self) -> Result<(), DbError>`

## Regras

- Conexão via `mongodb::Client::connect(uri).await`.
- Use `collection_name()` da entidade como nome da coleção.
- `id` é armazenado como `bson::ObjectId` (ou `String`, dependendo da decisão de design).
- Depende de `gerax-core`, `mongodb`, `bson`, `futures`.
- Todos os métodos são async.
- Trate erros de conexão e converta para `thiserror`.
- Parâmetros de conexão e autenticação devem vir de arquivos/configuração externa.

## Casos de erro

- ConnectionError
- SerializationError
- NotFoundError

## Testes esperados

- Deve criar instância da estrutura RepositoryMongoDb.
- Deve fazer conexão.
- Deve criar uma entidade.
- Deve buscar uma entidade existente.
- Deve retornar None quando não existir.
- Property test: round-trip de entidade serializada em BSON.
