# gerax-cqrs

Infraestrutura genérica para execução de Commands e Queries (CQRS) no ecossistema Gerax.

## Estrutura

```
crates/gerax-cqrs/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── error.rs
    ├── message.rs
    ├── command.rs
    ├── query.rs
    ├── handler.rs
    ├── erased.rs
    ├── registry.rs
    ├── command_bus.rs
    └── query_bus.rs
```

## Documentação

Use `cargo doc --open -p gerax-cqrs` para ver a documentação gerada.

## API

### Mensagens

```rust
#[derive(gerax_macros::Command)]
#[command(output = Aluno)]
pub struct CreateAluno {
    pub nome: String,
    pub email: String,
}

#[derive(gerax_macros::Query)]
#[query(output = Option<Aluno>)]
pub struct GetAluno {
    pub id: u64,
}
```

### Handlers

```rust
#[async_trait]
impl CommandHandler for CreateAlunoHandler {
    type Command = CreateAluno;

    async fn handle(&self, command: CreateAluno) -> Result<Aluno, CqrsError> {
        // ...
    }
}
```

### Registry e Buses

```rust
let mut registry = HandlerRegistry::new();
register_commands!(registry, CreateAlunoHandler::new(repo.clone()));
register_queries!(registry, GetAlunoHandler::new(repo.clone()));

let registry = Arc::new(registry);
let command_bus = CommandBus::new(registry.clone());
let query_bus = QueryBus::new(registry);

let aluno = command_bus.dispatch(CreateAluno { .. }).await?;
let aluno = query_bus.execute(GetAluno { id: 1 }).await?;
```

## Exemplo

- [`examples/cqrs-demo`](../../examples/cqrs-demo) — fluxo completo `CreateAluno`/`GetAluno`.

## Exemplo completo

Veja [`crates/gerax-cqrs/tests/cqrs_flow.rs`](tests/cqrs_flow.rs) para testes de integração.