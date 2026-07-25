# gerax-db

Crate de abstração de persistência do framework Gerax.

## Dependências

```toml
gerax-db = { path = "../crates/gerax-db" }
```

## Visão Geral

Define as interfaces assíncronas de conexão e repositório usadas por crates
mais específicos, como `gerax-mongodb` e `gerax-postgres`.

```
Connection -> Repository<T> -> DbError
```

## API

### Connection

Representa uma conexão assíncrona com um banco de dados.

```rust
pub trait Connection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized;

    async fn ping(&self) -> Result<(), DbError>;
}
```

---

### Repository<T>

Repositório genérico de entidades.

```rust
pub trait Repository<T: Entity + Send + Sync + 'static> {
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, DbError>;
    async fn find_all(&self) -> Result<Vec<T>, DbError>;
    async fn insert(&self, entity: T) -> Result<T, DbError>;
    async fn update(&self, entity: T) -> Result<(), DbError>;
    async fn delete(&self, id: &str) -> Result<(), DbError>;
}
```

---

### DbError

Erros padrão do crate.

```rust
pub enum DbError {
    ConnectionError(std::io::Error),
    SerializationError(String),
    NotFoundError(String),
}
```

Construtores auxiliares:

- `connection<E: fmt::Display>(err: E) -> Self`
- `serialization<E: fmt::Display>(err: E) -> Self`
- `not_found<E: fmt::Display>(id: E) -> Self`

---

### Erros

`Result<T, DbError>`

Variações de `DbError`:

- `ConnectionError(std::io::Error)` — falha de conexão com o banco.
- `SerializationError(String)` — erro ao serializar/deserializar dados.
- `NotFoundError(String)` — entidade não encontrada para o identificador informado.

---

## Exemplos

```rust
use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::{Connection, DbError, Repository};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    name: String,
}

impl Entity for User {
    fn id(&self) -> &str {
        &self.id
    }
}

#[async_trait]
impl Connection for MyDb {
    async fn connect() -> Result<Self, DbError> {
        Ok(MyDb { /* ... */ })
    }

    async fn ping(&self) -> Result<(), DbError> {
        Ok(())
    }
}

#[async_trait]
impl Repository<User> for MyRepo {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, DbError> {
        // TODO: implementar
        Ok(None)
    }

    async fn find_all(&self) -> Result<Vec<User>, DbError> {
        Ok(vec![])
    }

    async fn insert(&self, entity: User) -> Result<User, DbError> {
        Ok(entity)
    }

    async fn update(&self, entity: User) -> Result<(), DbError> {
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DbError> {
        Ok(())
    }
}
```
