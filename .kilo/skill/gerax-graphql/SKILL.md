---
name: gerax-graphql
description: Implementa a crate gerax-graphql seguindo o plano.md
---

# Skill: gerax-graphql

Use quando precisar implementar ou modificar o suporte a GraphQL na crate `gerax-graphql`.

## Especificação

Leia `crates/gerax-graphql/plano.md` antes de implementar.

## Visão geral

O crate `gerax-graphql` fornece suporte completo a GraphQL para o framework Gerax, com:

- Independente de Actix, Axum e Poem
- Integrado ao `gerax-http`
- Integrado ao `gerax-auth`
- Integrado ao `Context<State>`
- Arquitetura orientada a traits
- APIs públicas estáveis
- Baixo acoplamento
- Alta testabilidade
- Zero dependência pública de `async-graphql` (uso interno apenas)

## Arquitetura

```
Application
      │
      ▼
gerax-graphql
      │
      ▼
GraphQL Abstraction
      │
      ▼
Executor
      │
      ▼
async-graphql
      │
      ▼
gerax-http
```

## Contratos obrigatórios

### `GraphqlRequest`

```rust
pub struct GraphqlRequest {
    pub query: String,
    pub operation_name: Option<String>,
    pub variables: Option<serde_json::Map<String, serde_json::Value>>,
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}
```

Implementa `Serialize`, `Deserialize`, `Default`, `Clone`, `Debug`.

### `GraphqlResponse`

```rust
pub struct GraphqlResponse {
    pub data: Option<serde_json::Value>,
    pub errors: Option<Vec<GraphqlError>>,
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}
```

### `GraphqlError`

Enum com variantes:

- `Validation(String)` — erro de validação de schema, query ou argumentos
- `Execution(String)` — erro durante a execução de um resolver
- `Unauthorized(String)` — erro de autenticação
- `Forbidden(String)` — erro de autorização
- `PersistedQuery(String)` — erro de query persistida
- `ComplexityExceeded(String)` — complexidade da query excedeu o limite
- `DepthExceeded(String)` — profundidade da query excedeu o limite
- `Internal(String)` — erro interno do servidor

Implementa `Display`, `Error`, `Serialize`.

### `Resolver<State>`

Trait com método:

```rust
async fn resolve(&self, ctx: &Context<State>) -> Result<serde_json::Value, GraphqlError>;
```

### `Executor<State>`

Trait responsável por executar operações GraphQL:

```rust
async fn execute(&self, request: GraphqlRequest, state: &State) -> Result<GraphqlResponse, GraphqlError>;
```

Nunca depender diretamente de HTTP.

### `SchemaBuilder` e `Schema`

Builder pattern para construção de schemas GraphQL:

```rust
let schema = Schema::builder()
    .query(QueryRoot)
    .mutation(MutationRoot)
    .subscription(SubscriptionRoot)
    .finish()?;
```

### `GraphqlHandler`

Handler que implementa `Handler<State>` do `gerax-http`:

```rust
Router::new()
    .post("/graphql", graphql(schema))
```

### `GraphiQL` e `Playground`

```rust
GraphiQL::new("/graphql")
```

### `DataLoader`

Implementação compatível com `Context<State>` para batch loading.

## Regras

- Rust Edition 2024
- Todo código deve possuir documentação
- Todo item público deve possuir exemplos
- Nenhum `unwrap`, `expect` ou `panic`
- Todos os erros devem usar `Result`
- Todos os módulos devem possuir testes
- Cobertura mínima de 95%
- Dependências internas permitidas: `async-graphql`, `serde`, `serde_json`, `futures`, `tokio`
- Nunca expor essas dependências na API pública
- Nunca depender de Actix, Axum ou Poem diretamente
- Apenas `gerax-http` para integração HTTP
- Integração transparente com `gerax-auth`
- Usar `Arc` para compartilhamento de estado
- Evitar alocações desnecessárias e cópias desnecessárias
- Zero cópias desnecessárias

## Módulos

| Módulo       | Descrição |
|--------------|-----------|
| `builder.rs` | Builder de schema |
| `schema.rs` | Tipos de schema |
| `request.rs` | `GraphqlRequest` |
| `response.rs` | `GraphqlResponse` |
| `error.rs` | `GraphqlError` |
| `executor.rs` | `Executor<State>` trait |
| `context.rs` | Contexto GraphQL |
| `handler.rs` | `GraphqlHandler` |
| `subscription.rs` | Suporte a subscriptions |
| `middleware.rs` | Middleware GraphQL |
| `graphiql.rs` | UI GraphiQL |
| `playground.rs` | UI Playground |
| `scalar.rs` | Tipos escalares customizados |
| `resolver/` | Traits e implementações de resolver |
| `resolver/query.rs` | Resolvers de query |
| `resolver/mutation.rs` | Resolvers de mutation |
| `resolver/subscription.rs` | Resolvers de subscription |
| `resolver/dataloader.rs` | DataLoader |
| `extensions/` | Extensões GraphQL |
| `extensions/validation.rs` | Validação de schema, query e argumentos |
| `extensions/complexity.rs` | Limite de complexidade |
| `extensions/depth.rs` | Limite de profundidade |
| `extensions/persisted_query.rs` | Queries persistidas (SHA256 + cache) |
| `extensions/cache.rs` | Cache de respostas |
| `extensions/introspection.rs` | Controle de introspection |
| `tests/` | Testes de integração |

## Casos de erro

- ValidationError
- ExecutionError
- UnauthorizedError
- ForbiddenError
- PersistedQueryError
- ComplexityExceededError
- DepthExceededError
- InternalError
- ConnectionError (do gerax-db)
- NotFoundError (do gerax-db)

## Testes esperados

- Deve criar uma instância de `SchemaBuilder`.
- Deve construir um schema válido via `build()`.
- Deve criar uma instância de `GraphqlRequest`.
- Deve criar uma instância de `GraphqlResponse`.
- Deve criar uma instância de `GraphqlError` para cada variante.
- Deve executar uma query válida e retornar resultado JSON.
- Deve retornar `GraphqlError::ValidationError` para query com sintaxe inválida.
- Deve retornar `GraphqlError::ExecutionError` quando um resolver falhar.
- Deve retornar `GraphqlError::Unauthorized` quando auth falhar.
- Deve retornar `GraphqlError::ComplexityExceeded` quando complexidade exceder limite.
- Deve retornar `GraphqlError::DepthExceeded` quando profundidade exceder limite.
- Deve integrar com `gerax-db::Repository<T>` para buscar dados de entidades.
- Deve integrar com `gerax-auth` para claims de autenticação.
- Deve implementar `Handler<State>` para integração com `gerax-http`.
- Deve suportar subscriptions via WebSocket.
- Deve suportar persisted queries com SHA256.
- Deve suportar DataLoader para batch loading.
- Deve suportar middleware para logging, tracing, metrics, cache e authorization.
- Deve suportar GraphiQL e Playground UI.
- Deve ter cobertura superior a 95%.

## Critérios de aceitação

- Todo código compila sem warnings
- Clippy sem avisos
- rustfmt aplicado
- Todos os testes aprovados
- Cobertura superior a 95%
- API pública independente de async-graphql
- Integração transparente com gerax-http
- Integração transparente com gerax-auth
- Compatível com Actix, Axum e Poem através dos adaptadores do Gerax
- Código modular e preparado para futuras implementações (Federation, Apollo Router, GraphQL over HTTP, GraphQL over SSE) sem mudanças na API pública

## Tarefas independentes

1. **Core types**: `GraphqlRequest`, `GraphqlResponse`, `GraphqlError`
2. **SchemaBuilder e Schema**: Builder pattern para construção de schemas
3. **Executor trait**: Trait `Executor<State>` e implementação
4. **Resolver trait**: Trait `Resolver<State>` e implementação
5. **Context e State**: Gerenciamento de contexto e estado
6. **Handler**: `GraphqlHandler` implementando `Handler<State>` do `gerax-http`
7. **Middleware**: Middleware específico para GraphQL
8. **Subscriptions**: Suporte a subscriptions com WebSocket
9. **GraphiQL e Playground**: UIs de desenvolvimento
10. **Persisted Queries**: SHA256 + cache interno
11. **Validation**: Validação de schema, query e argumentos
12. **Complexity e Depth**: Limites configuráveis
13. **Introspection**: Toggle de habilitação/desabilitação
14. **DataLoader**: Implementação de batch loading
15. **Auth**: Integração com `gerax-auth`
16. **Extensions**: Módulo de extensões (validation, complexity, depth, persisted_query, cache)
17. **Scalar types**: Tipos escalares customizados
18. **Testes**: Testes para todos os componentes
19. **Benchmarks**: Benchmarks para Schema, Executor, Resolver, Subscriptions, DataLoader
20. **Documentação e qualidade**: Documentação completa, clippy, rustfmt, zero warnings