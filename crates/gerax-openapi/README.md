# gerax-openapi

Suporte a OpenAPI no ecossistema Gerax.

## Estrutura

```
crates/gerax-openapi/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    └── openapi/
        ├── mod.rs
        └── builder.rs
```

## Documentação

Use `cargo doc --open -p gerax-openapi` para ver a documentação gerada.

## Uso futuro

Este crate fornecerá:
- Geração de especificações OpenAPI a partir de entidades `gerax-core`
- Documentação automática da API
- Integração com `gerax-http` para expor o endpoint de documentação OpenAPI
- Integração com `gerax-db` para refletir schemas de entidades