# Adicionar crate
- create directory in crates
- create file Cargo.toml
- create directory src
- crate directory src/lib

# Adicionar conteudo no arquivo Cargo.toml
```toml
[package]
name = "##crate-name" # gerax-name

version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish.workspace = true

description = "Integração do Gerax com ##crate_name"

[dependencies]
gerax-core.workspace = true
gerax-db.workspace = true

serde.workspace = true
serde_json.workspace = true
```
# Adicionar referencia no Cargo.toml raiz
```
members = [
    ...
"crates/gerax-##name",
    ...
]


[workspace.dependencies]
    ...
gerax = { path = "crates/gerax-##name" }
    ...

```
# Adicionar referencia no Cargo.toml do crate facada gerax    
```
[dependencies]
    ...
gerax-##name = { workspace = true, optional = true }
    ...

[features]
    ...
name = [""dep:gerax-name"]
    ...


```