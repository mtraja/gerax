# Adicionar crate

## 1. Criar estrutura de diretórios

```bash
mkdir -p crates/gerax-<nome>/src/<module>
```

A estrutura de diretórios segue o padrão dos crates existentes:

```
crates/gerax-<nome>/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    └── <module>/
        ├── mod.rs
        └── <module>.rs
```

## 2. Criar `crates/gerax-<nome>/Cargo.toml`

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

description = "Integração do Gerax com <nome>"

[dependencies]
gerax-core.workspace = true
gerax-http.workspace = true
gerax-db.workspace = true

async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
```

Adaptar as dependências conforme necessário (ex: remover `gerax-http` se não for necessário).

## 3. Criar `crates/gerax-<nome>/README.md`

Estrutura padrão:

```markdown
# gerax-<nome>

Suporte a <nome> no ecossistema Gerax.

## Estrutura

\`\`\`
crates/gerax-<nome>/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    └── <module>/
        ├── mod.rs
        └── <module>.rs
\`\`\`

## Documentação

Use \`cargo doc --open -p gerax-<nome>\` para ver a documentação gerada.

## Uso futuro

Este crate fornecerá:
- <Funcionalidade principal>
- Integração com \`gerax-db\` para resolver dados
- Integração com \`gerax-http\` para expor endpoints
```

## 4. Criar `crates/gerax-<nome>/src/lib.rs`

```rust
//! # gerax-<nome>
//!
//! Suporte a <nome> no ecossistema Gerax.
//!
//! Este crate fornece:
//! - <Funcionalidade principal>
//! - Integração com `gerax-db` para resolver dados
//! - Integração com `gerax-http` para expor endpoints

pub mod <module>;
```

## 5. Criar arquivos do módulo

### `src/<module>/mod.rs`

```rust
//! Descrição do módulo <module>.

pub mod <module>;
```

### `src/<module>/<module>.rs`

Implementar as structs e traits do módulo seguindo os padrões do projeto.

## 6. Registrar no workspace root `Cargo.toml`

Adicionar ao `members`:

```toml
"crates/gerax-<nome>",
```

Adicionar às `[workspace.dependencies]`:

```toml
gerax-<nome> = { path = "crates/gerax-<nome>" }
```

## 7. Integrar na fachada `gerax` (opcional)

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

## 8. Verificar

```bash
cargo check -p gerax-<nome>
cargo check -p gerax --features <nome>
```