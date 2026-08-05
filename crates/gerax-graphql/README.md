# gerax-graphql

`gerax-graphql` adiciona GraphQL ao framework Gerax sem acoplamento direto a
adaptadores HTTP. O crate integra schema, executor, handler HTTP, autenticação,
subscriptions, DataLoader e extensões de execução.

## Componentes

- `Schema` e `SchemaBuilder` encapsulam a execução GraphQL.
- `GraphqlHandler` implementa o handler do `gerax-http`.
- `GraphqlRequest`, `GraphqlResponse` e `GraphqlError` definem a API de transporte.
- `GraphqlAuthContext` e `AuthMiddleware` usam as claims produzidas pelo
  `gerax-auth` e permitem exigir escopos.
- `DataLoader` agrupa carregamentos assíncronos e mantém cache por contexto.
- `SubscriptionManager` e `WebSocketSubscriptionAdapter` oferecem a abstração de
  subscriptions.
- As extensões incluem validação, limites de complexidade e profundidade,
  introspection, queries persistidas e cache de respostas.
- Os escalares `UUID`, `DateTime` (RFC 3339) e `Email` validam valores de entrada.

## Uso

Um schema é construído com os tipos raiz da aplicação. `async-graphql` é usado
internamente pelo crate para definir os tipos raiz e executar a operação.

```rust,ignore
let schema = Schema::builder()
    .query(QueryRoot)
    .mutation(MutationRoot)
    .subscription(SubscriptionRoot)
    .finish()?;
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
