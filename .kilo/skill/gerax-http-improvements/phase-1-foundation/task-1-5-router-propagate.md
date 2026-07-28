# Tarefa 1.5: Atualizar `Router` para propagar erros da chain de middleware

## Fase
Fase 1 — Fundação

## Objetivo
Garantir que o `Router` propague erros de middlewares e handlers corretamente ao executar requisições. `Router::handle()` deve retornar `ServerResult<Response>`, e erros de qualquer etapa da cadeia devem ser retornados ao caller.

## Arquivos Afetados
- `crates/gerax-http/src/routing/router.rs`

## Instruções Detalhadas

### Passo 1: Adicionar método `handle` ao `Router`

Em `crates/gerax-http/src/routing/router.rs`, adicione o método `handle` que executa a cadeia de middleware + handler para uma requisição:

```rust
impl<State> Router<State> {
    pub async fn handle(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        // Busca a route correspondente pelo método e path
        if let Some(route) = self.find_route(&ctx) {
            route.execute(ctx).await
        } else {
            Err(HttpServerError::HandlerError("Route not found".to_string()))
        }
    }

    fn find_route(&self, ctx: &Context<State>) -> Option<&Route<State>> {
        let path = ctx.request().path();
        let method = ctx.request().method();

        self.routes.iter().find(|route| {
            route.method() == *method && route.path() == path
        })
    }
}
```

### Passo 2: Atualizar `Scope` para propagar erros

Em `crates/gerax-http/src/routing/scope.rs`, adicione método `handle` que executa a cadeia de middleware do scope + route:

```rust
impl<State> Scope<State> {
    pub async fn handle(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        let path = ctx.request().path();
        let method = ctx.request().method();

        // Verifica se o path começa com o prefixo do scope
        if !path.starts_with(&self.prefix) {
            return Err(HttpServerError::HandlerError(
                "Path does not match scope prefix".to_string(),
            ));
        }

        // Busca a route correspondente dentro do scope
        if let Some(route) = self.routes.iter().find(|route| {
            route.method() == *method && route.path() == path
        }) {
            // Executa middlewares do scope + middlewares da route + handler
            let all_middlewares = self
                .middlewares
                .iter()
                .chain(route.middlewares().iter())
                .cloned()
                .collect();

            let next = Next::new_with_chain(ctx, all_middlewares, Arc::clone(route.handler()));
            if let Some(middleware) = all_middlewares.first() {
                middleware.handle(ctx, next).await
            } else {
                route.handler().call(ctx).await
            }
        } else {
            // Verifica sub-scopes
            for scope in &self.scopes {
                return scope.handle(ctx).await;
            }

            Err(HttpServerError::HandlerError("Route not found in scope".to_string()))
        }
    }
}
```

### Passo 3: Atualizar imports necessários

Em `router.rs`, adicione os imports necessários:

```rust
use crate::{Context, HttpServerError, ServerResult};
use crate::Response;
use std::sync::Arc;
```

Em `scope.rs`, adicione os imports necessários:

```rust
use crate::{Context, HttpServerError, Next, ServerResult};
use crate::Response;
use std::sync::Arc;
```

### Passo 4: Verificar compilação

Execute `cargo check -p gerax-http` para garantir que todas as mudanças compilam sem erros nem warnings.

### Passo 5: Atualizar `lib.rs` se necessário

Se `Next` precisa ser exportado para uso externo, adicione-o aos re-exports em `crates/gerax-http/src/lib.rs`:

```rust
pub use middleware::Next;
```

## Código de Referência

### `router.rs` — estado final esperado (parcial, apenas `handle` e `find_route`)

```rust
impl<State> Router<State> {
    pub async fn handle(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        if let Some(route) = self.find_route(&ctx) {
            route.execute(ctx).await
        } else {
            Err(HttpServerError::HandlerError("Route not found".to_string()))
        }
    }

    fn find_route(&self, ctx: &Context<State>) -> Option<&Route<State>> {
        let path = ctx.request().path();
        let method = ctx.request().method();

        self.routes.iter().find(|route| {
            route.method() == *method && route.path() == path
        })
    }
}
```

## Critérios de Aceite

- [ ] `Router::handle()` retorna `ServerResult<Response>`
- [ ] Erro em middleware é propagado até o caller
- [ ] Erro em handler é propagado até o caller
- [ ] 404 (route não encontrada) retorna `HttpServerError::HandlerError`
- [ ] `cargo check -p gerax-http` passa sem erros nem warnings
- [ ] `Next` é exportado em `lib.rs` se necessário para uso externo

## Dependências

- Tarefa 1.4 (`Route::execute()` e `Next` com encadeamento)

## Notas

- O `Router::handle()` é o ponto de entrada para processar uma requisição HTTP. Ele encontra a route correspondente e executa a cadeia completa de middleware + handler.
- Erros de qualquer etapa (middleware ou handler) são retornados como `Err(HttpServerError)`, permitindo que o `HttpServer` os trate de forma centralizada.
- O `Scope::handle()` delega para sub-scopes recursivamente, permitindo nesting ilimitado de scopes.