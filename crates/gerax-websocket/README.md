# gerax-websocket

Suporte a WebSocket no ecossistema Gerax.

## Estrutura

```
crates/gerax-websocket/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    └── websocket/
        ├── mod.rs
        └── websocket.rs
```

## Documentação

Use `cargo doc --open -p gerax-websocket` para ver a documentação gerada.

## Uso futuro

Este crate fornecerá:
- Servidor WebSocket genérico
- Cliente WebSocket genérico
- Integração com `gerax-db` para resolver dados das entidades
- Integração com `gerax-http` para expor endpoints WebSocket