# gerax

Framework Gerax.

## Dependências

```toml
gerax = { path = "../crates/gerax" }
```

## Visão Geral

Crate fachada do Gerax. Reexporta tipos e funcionalidades dos crates do
ecossistema condicionalmente por features, oferecendo uma integração
direta via feature flags.

## Features

| Feature | Descrição |
|---------|-----------|
| `actix` | Suporte a Actix Web via `gerax-actix` |
| `axum` | Suporte a Axum via `gerax-axum` |
| `poem` | Suporte a Poem via `gerax-poem` |
| `mongodb` | Suporte a MongoDB via `gerax-mongodb` |
| `postgres` | Suporte a PostgreSQL via `gerax-postgres` |
| `turso` | Suporte a Turso via `gerax-turso` |
| `ai` | Integração de IA via `gerax-ai` |
| `mysql` | Suporte a MySQL via `gerax-mysql` |
| `auth` | Autenticação/autorização via `gerax-auth` |
| `config` | Configuração via `gerax-config` |

Feature default: `actix`, `mongodb`.

```toml
gerax = { path = "../crates/gerax", features = ["actix", "config"] }
```

## API

### Core público

Reexportações baseadas em `gerax-core`:

- `Entity`

### Macros públicos

Reexportações baseadas em `gerax-macros`:

- Proc-macros disponíveis quando `gerax-macros` é habilitado.

### Reexportações condicionais

A fachada reexporta tudo dos crates abaixo quando a respectiva feature
estiver ativa:

- `actix` -> `gerax_actix::*`
- `axum` -> `gerax_axum::*`
- `poem` -> `gerax_poem::*`
- `mongodb` -> `gerax_mongodb::*`
- `postgres` -> `gerax_postgres::*`
- `turso` -> `gerax_turso::*`
- `ai` -> `gerax_ai::*`
- `mysql` -> `gerax_mysql::*`
- `auth` -> `gerax_auth::*`
- `config` -> `gerax_config::*`

### Prelude

`gerax::prelude` reexporta os tipos mais comuns por feature, reduzindo
importações.

---

## Exemplos

```rust
use gerax::Entity;

#[derive(gerax_macros::Entity)]
struct User {
    id: Option<String>,
    name: String,
}

fn main() {
    println!("Entidade base pronta.");
}
```
