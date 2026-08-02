# gerax-graphql

Suporte a GraphQL no ecossistema Gerax.

## Estrutura

```
crates/gerax-graphql/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── schema/
    │   ├── mod.rs
    │   └── builder.rs
    ├── executor/
    │   ├── mod.rs
    │   └── executor.rs
    └── error/
        └── mod.rs
```

## Uso futuro

Este crate fornecerá:
- Construção de schema GraphQL a partir de entidades `gerax-core`
- Execução de queries/mutations
- Integração com `gerax-db` para resolver dados
- Integração com `gerax-http` para expor endpoint GraphQL
