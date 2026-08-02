# gerax-grpc

Suporte a gRPC no ecossistema Gerax.

## Estrutura

```
crates/gerax-grpc/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── proto/
    │   ├── mod.rs
    │   └── builder.rs
    ├── server/
    │   ├── mod.rs
    │   └── server.rs
    ├── client/
    │   ├── mod.rs
    │   └── client.rs
    └── error/
        └── mod.rs
```

## Documentação

Use `cargo doc --open -p gerax-grpc` para ver a documentação gerada.

## Uso futuro

Este crate fornecerá:
- Geração de schemas Protobuf a partir de entidades `gerax-core`
- Servidor gRPC genérico
- Cliente gRPC genérico
- Integração com `gerax-db` para resolver dados
- Integração com `gerax-http` para expor endpoints gRPC
