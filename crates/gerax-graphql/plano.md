# Skill: Implementar o crate `gerax-graphql`

## Objetivo

Implementar um crate chamado **gerax-graphql** para o framework Gerax.

O crate deve fornecer suporte completo a GraphQL, mantendo a filosofia arquitetural do Gerax:

* Independente de Actix, Axum e Poem.
* Integrado ao `gerax-http`.
* Integrado ao `gerax-auth`.
* Integrado ao `Context<State>`.
* Arquitetura orientada a traits.
* APIs públicas estáveis.
* Baixo acoplamento.
* Alta testabilidade.
* Zero dependência pública de async-graphql.

A implementação interna pode utilizar `async-graphql`, porém nenhum tipo dessa biblioteca deve aparecer na API pública.

---

# Arquitetura

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

---

# Estrutura do projeto

```
gerax-graphql/

src/

lib.rs

builder.rs

schema.rs

request.rs

response.rs

error.rs

executor.rs

context.rs

handler.rs

subscription.rs

middleware.rs

graphiql.rs

playground.rs

scalar.rs

resolver/

mod.rs

query.rs

mutation.rs

subscription.rs

dataloader.rs

extensions/

validation.rs

complexity.rs

depth.rs

persisted_query.rs

cache.rs

introspection.rs

tests/
```

---

# Requisitos

Utilizar Rust Edition 2024.

Todo código deve possuir documentação.

Todos os itens públicos devem possuir exemplos.

Nenhum unwrap.

Nenhum expect.

Nenhum panic.

Todos erros devem utilizar Result.

Todos módulos devem possuir testes.

Cobertura mínima:

95%

---

# Dependências internas

Pode utilizar internamente

* async-graphql
* serde
* serde_json
* futures
* tokio

Nunca expor essas dependências na API pública.

---

# API pública

A API deve parecer que GraphQL foi desenvolvido dentro do Gerax.

Exemplo:

```rust
let schema = Schema::builder()
    .query(QueryRoot)
    .mutation(MutationRoot)
    .subscription(SubscriptionRoot)
    .finish()?;
```

Nunca expor:

```
async_graphql::Schema
```

---

# Request

Criar

```rust
pub struct GraphqlRequest
```

Campos

```
query

operation_name

variables

extensions
```

Implementar

```
Serialize

Deserialize

Default

Clone

Debug
```

---

# Response

Criar

```rust
pub struct GraphqlResponse
```

Campos

```
data

errors

extensions
```

---

# Error

Criar

```rust
enum GraphqlError
```

Com variantes

```
Validation

Execution

Unauthorized

Forbidden

PersistedQuery

ComplexityExceeded

DepthExceeded

Internal
```

Implementar

```
Display

Error

Serialize
```

---

# Context

Reutilizar

```
Context<State>
```

Nunca criar um segundo contexto.

Adicionar extensões apenas quando necessário.

---

# Resolver

Criar trait

```rust
Resolver<State>
```

com

```rust
async fn resolve(...)
```

---

# Executor

Criar trait

```rust
Executor<State>
```

Responsável por executar uma operação GraphQL.

Nunca depender diretamente de HTTP.

---

# Schema

Criar

```
SchemaBuilder

Schema
```

Builder Pattern.

---

# Handler

Criar

```
GraphqlHandler
```

Esse handler deve implementar

```
Handler<State>
```

do gerax-http.

Assim GraphQL será apenas mais uma rota.

Exemplo

```rust
Router::new()
.post("/graphql", graphql(schema))
```

---

# Middleware

Criar middleware específico para GraphQL.

Interface semelhante ao middleware HTTP.

Permitir:

Logging

Tracing

Metrics

Cache

Authorization

Persisted Queries

---

# Subscription

Implementar subscriptions.

Utilizar WebSocket.

Criar abstração independente do framework HTTP.

Cada adapter será responsável por conectar ao servidor.

---

# GraphiQL

Criar

```
GraphiQL
```

Com

```
GraphiQL::new("/graphql")
```

---

# Playground

Criar

```
Playground
```

---

# Persisted Queries

Implementar suporte.

Hash SHA256.

Cache interno.

---

# Validation

Implementar

Validação do schema.

Validação da query.

Validação dos argumentos.

---

# Complexity

Implementar limite configurável.

---

# Depth

Implementar limite configurável.

---

# Introspection

Permitir habilitar/desabilitar.

---

# DataLoader

Implementar

```
DataLoader
```

Compatível com

```
Context<State>
```

---

# Auth

Integrar com

```
gerax-auth
```

Exemplo

```rust
let claims = ctx.auth::<Claims>()?;
```

---

# Integração HTTP

Nunca depender de

Actix

Axum

Poem

Apenas do

```
gerax-http
```

---

# Performance

Evitar alocações.

Utilizar Arc.

Evitar Box quando possível.

Zero cópias desnecessárias.

---

# Benchmarks

Criar benchmarks para

Schema

Executor

Resolver

Subscriptions

DataLoader

---

# Testes

Criar testes para

Queries

Mutations

Subscriptions

Errors

Validation

Complexity

Depth

Persisted Queries

Middleware

DataLoader

Handler

Executor

Schema

---

# Documentação

Gerar documentação completa.

Cada item público deve possuir:

Descrição.

Exemplo.

Erros possíveis.

Complexidade.

---

# Critérios de aceitação

A implementação será considerada concluída apenas quando:

* Todo código compilar sem warnings.
* Clippy sem avisos.
* rustfmt aplicado.
* Todos os testes aprovados.
* Cobertura superior a 95%.
* API pública independente de async-graphql.
* Integração transparente com gerax-http.
* Integração transparente com gerax-auth.
* Compatível com Actix, Axum e Poem através dos adaptadores do Gerax.
* Código modular e preparado para futuras implementações como Federation, Apollo Router, GraphQL over HTTP e GraphQL over SSE sem necessidade de mudanças na API pública.
