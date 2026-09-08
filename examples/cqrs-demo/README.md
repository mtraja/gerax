# cqrs-demo

Exemplo de integração do `gerax-cqrs` com as macros derivadas.

Demonstra o fluxo completo:

```
CreateAluno → CreateAlunoHandler → CommandBus → Aluno
GetAluno    → GetAlunoHandler    → QueryBus   → Option<Aluno>
```

## Estrutura

```
examples/cqrs-demo/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs           // ponto de entrada
    ├── bootstrap.rs      // composition root (registry + buses)
    ├── application.rs    // handlers e port de repository
    └── domain.rs         // entidades e mensagens (Command/Query)
```

## Executar

```bash
cargo run -p cqrs-demo
```