# gerax-capnp

Suporte a Cap'n Proto no ecossistema Gerax.

## Estrutura

```
crates/gerax-capnp/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    └── capnp/
        ├── mod.rs
        └── builder.rs
```

## Documentação

Use `cargo doc --open -p gerax-capnp` para ver a documentação gerada.

## Uso futuro

Este crate fornecerá:
- Geração de schemas Cap'n Proto a partir de entidades `gerax-core`
- Serialização/deserialização de mensagens Cap'n Proto
- Integração com `gerax-db` para persistir dados em formato Cap'n Proto
- Integração com `gerax-http` para expor endpoints Cap'n Proto