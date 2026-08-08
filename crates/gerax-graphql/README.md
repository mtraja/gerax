# gerax-graphql

`gerax-graphql` adiciona GraphQL ao framework Gerax sem acoplamento direto a
adaptadores HTTP. O crate integra schema, executor, handler HTTP, autenticação,
subscriptions, DataLoader e extensões de execução.

## Componentes

### Core
- `Schema` e `SchemaBuilder` encapsulam a construção e execução GraphQL.
- `Executor` e `DefaultExecutor` definem a trait de execução e implementação padrão.
- `GraphqlHandler` implementa o handler do `gerax-http`.
- `GraphqlRequest`, `GraphqlResponse` e `GraphqlError` definem a API de transporte.

### Autenticação
- `GraphqlAuthContext` e `AuthMiddleware` usam as claims produzidas pelo
  `gerax-auth` e permitem exigir escopos.
- `AuthClaims`, `AuthExtractor` e `AuthIntegration` para integração customizada.

### DataLoader
- `DataLoader` e `BatchLoader` agrupam carregamentos assíncronos e mantêm cache por contexto.

### Subscriptions
- `SubscriptionManager` e `WebSocketSubscriptionAdapter` oferecem a abstração de
  subscriptions via WebSocket.
- `Subscription` trait para definir subscription roots.

### Extensões
- **Validação:** `Validator`, `QueryValidator`, `SchemaValidator`, `ArgumentsValidator`.
- **Limites:** `ComplexityLimiter`, `DepthLimiter`.
- **Introspection:** `IntrospectionController`.
- **Queries persistidas:** `PersistedQueryManager`, `PersistedQueryCache`.
- **Cache:** `ResponseCache` via `CacheMiddleware`.

### Middlewares
- `AuthMiddleware` — autenticação e escopos.
- `CacheMiddleware` — cache de respostas.
- `LoggingMiddleware` — logging de requisições.
- `MetricsMiddleware` — coleta de métricas.
- `GraphqlMiddleware` — trait base para middlewares GraphQL.

### Escalares
- `UuidScalar` — valida e serializa `Uuid`.
- `DateTimeScalar` — RFC 3339.
- `EmailScalar` — valida emails.

### UI
- `GraphiQL` e `Playground` para interfaces de desenvolvimento.

## Uso

Um schema é construído com os tipos raiz da aplicação. `async-graphql` é usado
internamente pelo crate para definir os tipos raiz e executar a operação.

```rust,ignore
use gerax_graphql::{Schema, GraphqlQueryRoot, GraphqlMutationRoot};

let schema = Schema::builder()
    .query(QueryRoot)
    .mutation(MutationRoot)
    .subscription(SubscriptionRoot)
    .finish()?;
```

Para expor um endpoint HTTP, use `GraphqlHandler` com uma implementação
de `Executor`:

```rust,ignore
use gerax_graphql::{GraphqlHandler, DefaultExecutor};

let handler = GraphqlHandler::new(schema, DefaultExecutor);
```

Para recuperar credenciais autenticadas em código GraphQL, importe o trait de
contexto e leia as claims inseridas pelo middleware HTTP do `gerax-auth`:

```rust,ignore
use gerax_auth::Claims;
use gerax_graphql::GraphqlAuthContext;

let claims = ctx.auth::<Claims>()?;
```

## Qualidade e benchmarks

```bash
cargo fmt -p gerax-graphql --check
cargo test -p gerax-graphql
cargo clippy -p gerax-graphql --all-targets --no-deps -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p gerax-graphql --no-deps
cargo bench -p gerax-graphql --bench graphql_components
```

O benchmark mede a construção de schema e a execução de executor, resolver,
subscription e DataLoader. Use `GERAX_BENCH_ITERATIONS` para reduzir ou ampliar
o número de iterações.
