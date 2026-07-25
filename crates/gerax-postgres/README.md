# gerax-postgres

Suporte a PostgreSQL para o framework Gerax.

## Dependências

```toml
gerax-postgres = { path = "../crates/gerax-postgres" }
```

## Visão Geral

Implementa `Connection` e `Repository<T>` para PostgreSQL, integrando-se a
`gerax-db` e `gerax-core`. A configuração pode ser carregada a partir de
variáveis de ambiente.

```
PostgresConfig -> PostgresRepository<T> -> Connection / Repository<T>
```

## API

### PostgresConfig

Configuração de conexão com o PostgreSQL.

```rust
pub struct PostgresConfig {
    pub uri: String,
    pub database: String,
}
```

Métodos:

- `from_env() -> Result<Self, DbError>` — lê `DATABASE_URL` e `DATABASE_NAME` do ambiente.

---

### PostgresRepository<T>

Repositório PostgreSQL para entidades que implementam `gerax_core::Entity`.

```rust
pub struct PostgresRepository<T> { /* ... */ }
```

Implementa:

- `Connection`
- `Repository<T>`

Operações suportadas:

- `find_by_id(id: &str) -> Result<Option<T>, DbError>`
- `find_all() -> Result<Vec<T>, DbError>`
- `insert(entity: T) -> Result<T, DbError>`
- `update(entity: T) -> Result<(), DbError>`
- `delete(id: &str) -> Result<(), DbError>`
- `connect() -> Result<Self, DbError>`
- `ping(&self) -> Result<(), DbError>`
- `create_table(&self) -> Result<(), DbError>`

---

### Erros

`Result<T, DbError>` _(definido em `gerax-db`)_

Variações de `DbError`:

- `ConnectionError(std::io::Error)`
- `SerializationError(String)`
- `NotFoundError(String)`

---

## Exemplos

```rust
use gerax_core::Entity;
use gerax_db::Repository;
use gerax_postgres::{PostgresConfig, PostgresRepository};

#[derive(serde::Serialize, serde::Deserialize, Entity)]
struct User {
    id: Option<String>,
    name: String,
}

#[async_trait::async_trait]
impl Entity for User {
    fn collection_name() -> &'static str {
        "users"
    }
}

async fn example() -> gerax_db::DbResult {
    let repo = PostgresRepository::<User>::connect().await?;
    repo.ping().await?;
    repo.create_table().await?;

    let user = repo.find_by_id("...").await?;
    let all = repo.find_all().await?;

    Ok(())
}
```
