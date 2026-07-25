# gerax-mysql

Suporte a MySQL para o framework Gerax.

## Dependências

```toml
gerax-mysql = { path = "../crates/gerax-mysql" }
```

## Visão Geral

Implementa `Connection` e `Repository<T>` para MySQL, integrando-se a
`gerax-db` e `gerax-core`. A configuração pode ser carregada a partir de
variáveis de ambiente.

```
MySqlConfig -> MySqlRepository<T> -> Connection / Repository<T>
```

## API

### MySqlConfig

Configuração de conexão com o MySQL.

```rust
pub struct MySqlConfig {
    pub url: String,
    pub database: String,
}
```

Métodos:

- `from_env() -> Result<Self, DbError>` — lê `DATABASE_URL` e `DATABASE_NAME` do ambiente.

---

### MySqlRepository<T>

Repositório MySQL para entidades que implementam `gerax_core::Entity`.

```rust
pub struct MySqlRepository<T> { /* ... */ }
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
use gerax_mysql::{MySqlConfig, MySqlRepository};

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
    let repo = MySqlRepository::<User>::connect().await?;
    repo.ping().await?;

    let user = repo.find_by_id("...").await?;
    let all = repo.find_all().await?;

    Ok(())
}
```
