---
name: gerax-postgres-repository
description: Implementa o Repository para PostgreSQL na crate gerax-postgres seguindo o spec RepositoryPostgre.md
---

# Skill: gerax-postgres-repository

Use quando precisar implementar ou modificar o repositório PostgreSQL na crate `gerax-postgres`.

## Especificação

Leia `crates/gerax-postgres/RepositoryPostgre.md` e `crates/gerax-postgres/SPEC.md` antes de implementar.

## Contratos obrigatórios

Implemente `gerax-db::Repository<T>` para **PostgreSQL** via `tokio-postgres`:

- `find_by_id(id: &str) -> Result<Option<T>, DbError>`
- `find_all() -> Result<Vec<T>, DbError>`
- `insert(entity: T) -> Result<T, DbError>`
- `update(entity: T) -> Result<(), DbError>`
- `delete(id: &str) -> Result<(), DbError>`

Implemente também `gerax-db::Connection`:

- `connect() -> Result<Self, DbError>`
- `ping(&self) -> Result<(), DbError>`

## Regras

- Conexão via `tokio_postgres::connect(uri, TcpStream).await`.
- Use queries parametrizadas para evitar SQL injection.
- `id` pode ser `String` ou `Uuid`, dependendo do schema.
- Depende de `gerax-core`, `tokio-postgres`, `futures`.
- Suporte a feature `with-chrono-0_4` do `tokio-postgres`.
- Transações via `tokio_postgres::Transaction`.
- Todos os métodos são async e usam `async-trait`.
- Erros usam `gerax-db::DbError` ou `gerax-core::Error`.
- Parâmetros de conexão e autenticação devem vir de arquivos/configuração externa.

## Casos de erro

- ConnectionError
- SerializationError
- NotFoundError

## Testes esperados

- Deve criar instância da estrutura RepositoryPostgres.
- Deve fazer conexão.
- Deve criar uma entidade.
- Deve buscar uma entidade existente.
- Deve retornar None quando não existir.
