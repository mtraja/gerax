# gerax-core

Core do framework Gerax.

## Dependências

```toml
gerax-core = { path = "../crates/gerax-core" }
```

## Visão Geral

Fornece os tipos fundamentais compartilhados por todo o ecossistema Gerax.
Principalmente a trait `Entity`, contratos comuns e tipos base usados pelos
demais crates.

```
Entity -> gerax-db / gerax-mongodb / gerax-postgres / ...
```

## API

### Entity

Representa uma entidade de domínio persistível.

```rust
pub trait Entity: Serialize + for<'de> Deserialize<'de> + Send + Sync + Unpin + Clone + 'static {
    fn collection_name() -> &'static str;
    fn id(&self) -> Option<String>;
    fn set_id(&mut self, id: String);
}
```

Propósito: definir o nome da coleção/tabela e o identificador mutável
para repositórios genéricos.

---

## Exemplos

```rust
use gerax_core::Entity;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Option<String>,
    name: String,
}

impl Entity for User {
    fn collection_name() -> &'static str {
        "users"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}
```
