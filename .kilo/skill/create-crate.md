---
name: create-crate
description: Cria um novo crate no workspace gerax seguindo o padrão existente
---

# Skill: create-crate

Use quando precisar criar um novo crate no workspace Gerax.

## Passos

### 1. Criar a estrutura de diretórios

```bash
mkdir -p crates/<nome-do-crate>/src
```

### 2. Criar `crates/<nome-do-crate>/Cargo.toml`

Seguir o padrão dos crates existentes:

```toml
[package]
name = "gerax-<nome>"

version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish.workspace = true

description = "<Descrição do crate>"

[dependencies]
gerax-core.workspace = true
async-trait.workspace = true
serde.workspace = true
thiserror.workspace = true
```

### 3. Criar `crates/<nome-do-crate>/src/lib.rs`

```rust
#![allow(unused_imports)]

// Definir traits e tipos públicos do crate
```

### 4. Registrar no workspace root `Cargo.toml`

Adicionar ao `members`:

```toml
"crates/<nome-do-crate>",
```

Adicionar às `[workspace.dependencies]`:

```toml
gerax-<nome> = { path = "crates/<nome-do-crate>" }
```

### 5. Integrar na fachada `gerax` (opcional)

Em `crates/gerax/Cargo.toml`, adicionar como dependência opcional:

```toml
gerax-<nome> = { workspace = true, optional = true }
```

E adicionar uma feature:

```toml
<nome> = ["dep:gerax-<nome>"]
```

Em `crates/gerax/src/lib.rs`, adicionar o re-export condicional:

```rust
#[cfg(feature = "<nome>")]
pub use gerax_<nome>::*;
```

Em `crates/gerax/src/prelude.rs`, adicionar:

```rust
#[cfg(feature = "<nome>")]
pub use gerax_<nome>::{

};
```

### 6. Verificar

```bash
cargo check -p gerax-<nome>
cargo check -p gerax --features <nome>
```