# gerax-macros

Macros proc-macro do framework Gerax.

## Dependências

```toml
gerax-macros = { path = "../crates/gerax-macros" }
```

## Visão Geral

Implementa procedural macros usadas pelo framework, incluindo derivations e
codificação de contratos comuns para entidades.

## API

### Entity

Macro proc-macro derive para entidades.

Uso:

```rust
#[derive(Entity)]
struct Product {
    id: Option<String>,
    name: String,
}
```

Regras:

- Apenas structs com named fields.
- Campo `id` deve existir e ser `Option<String>`.
- Gera implementação de `gerax_core::Entity`.

Atributo opcional:

```rust
#[entity(collection_name = "custom_collection")]
struct Item { ... }
```

---

## Exemplos

```rust
use gerax_macros::Entity;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Entity)]
struct User {
    id: Option<String>,
    name: String,
}

fn main() {
    println!("collection = {}", User::collection_name());
    println!("id = {:?}", User)
        .id(&User { id: Some("1".into()), name: "A".into() })
        .id());
}
```
