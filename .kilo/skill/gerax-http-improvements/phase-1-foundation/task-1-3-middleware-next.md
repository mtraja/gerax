# Tarefa 1.3: Implementar `Middleware` com `Next<State>` (chain of responsibility)

## Fase
Fase 1 — Fundação

## Objetivo
Transformar `Middleware` de marker trait (apenas `fn name(&self) -> &str`) em chain of responsibility com `Next<State>`. A trait `Middleware` deve ter um método `handle` que recebe o contexto e um `Next` para continuar a cadeia.

## Arquivos Afetados
- `crates/gerax-http/src/middleware/mod.rs`
- `crates/gerax-http/src/routing/context.rs`

## Instruções Detalhadas

### Passo 1: Reescrever `Middleware` trait com `handle` e `Next`

Em `crates/gerax-http/src/middleware/mod.rs`, substitua o conteúdo atual:

```rust
use crate::{Context, ServerResult, Response};

/// Próximo elo na cadeia de middleware.
///
/// Chamar `next.call()` avança para o próximo middleware ou para o handler final.
pub struct Next<State> {
    state: Context<State>,
}

impl<State> Next<State> {
    /// Cria um novo `Next` com o contexto atual.
    pub fn new(context: Context<State>) -> Self {
        Self { state: context }
    }

    /// Avança para o próximo middleware na cadeia ou para o handler final.
    ///
    /// Este método será sobrescrito pelo `Router` no momento da execução.
    pub async fn call(self) -> ServerResult<Response> {
        unreachable!("Next::call deve ser sobrescrito pelo Router")
    }
}

/// Trait para middleware que intercepta e processa requisições.
///
/// Implemente `handle` para executar lógica pré e pós-processamento.
/// Retorne `Ok(Response)` para curto-circuito (não chamar `next.call()`)
/// ou chame `next.call()` para continuar a cadeia.
#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    /// Nome identificador do middleware (para logging e debug).
    fn name(&self) -> &str;

    /// Processa a requisição.
    ///
    /// - Retorne `Ok(Response)` para encerrar a requisição imediatamente (curto-circuito).
    /// - Chame `next.call().await` para continuar a cadeia de middleware/handler.
    async fn handle(
        &self,
        ctx: Context<State>,
        next: Next<State>,
    ) -> ServerResult<Response>;
}
```

### Passo 2: Garantir que `Context<State>` tem getters públicos

Em `crates/gerax-http/src/routing/context.rs`, confirme que `Context<State>` expõe os getters necessários para que `Next` e `Middleware` acessem state, request, params e extensions:

```rust
impl<State> Context<State> {
    pub fn state(&self) -> Arc<State> { ... }
    pub fn request(&self) -> &Request { ... }
    pub fn request_mut(&mut self) -> &mut Request { ... }
    pub fn params(&self) -> &PathParams { ... }
    pub fn params_mut(&mut self) -> &mut PathParams { ... }
    pub fn extensions(&self) -> &Extensions { ... }
    pub fn extensions_mut(&mut self) -> &mut Extensions { ... }
}
```

### Passo 3: Atualizar `Route` para usar `Middleware<State>`

Em `crates/gerax-http/src/routing/route.rs`, atualize o tipo de `middlewares` para usar `Arc<dyn Middleware<State>>`:

```rust
pub struct Route<State> {
    method: HttpMethod,
    path: String,
    handler: Arc<dyn Handler<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}
```

### Passo 4: Atualizar `Router` para usar `Middleware<State>`

Em `crates/gerax-http/src/routing/router.rs`, atualize o tipo de `middlewares` para usar `Arc<dyn Middleware<State>>`:

```rust
pub struct Router<State> {
    routes: Vec<Route<State>>,
    scopes: Vec<Scope<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}
```

### Passo 5: Atualizar `Scope` para usar `Middleware<State>`

Em `crates/gerax-http/src/routing/scope.rs`, atualize o tipo de `middlewares` para usar `Arc<dyn Middleware<State>>`:

```rust
pub struct Scope<State> {
    prefix: String,
    routes: Vec<Route<State>>,
    scopes: Vec<Scope<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}
```

### Passo 6: Verificar compilação

Execute `cargo check -p gerax-http` para garantir que todas as mudanças compilam.

## Código de Referência

### `middleware/mod.rs` — estado final esperado

```rust
use async_trait::async_trait;

use crate::{Context, ServerResult, Response};

pub struct Next<State> {
    state: Context<State>,
}

impl<State> Next<State> {
    pub fn new(context: Context<State>) -> Self {
        Self { state: context }
    }

    pub async fn call(self) -> ServerResult<Response> {
        unreachable!("Next::call deve ser sobrescrito pelo Router")
    }
}

#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    fn name(&self) -> &str;

    async fn handle(
        &self,
        ctx: Context<State>,
        next: Next<State>,
    ) -> ServerResult<Response>;
}
```

## Critérios de Aceite

- [ ] `Middleware` trait tem método `handle(&self, ctx, next) -> ServerResult<Response>`
- [ ] `Next<State>` struct existe com método `call(self) -> ServerResult<Response>`
- [ ] `Middleware` ainda é `Send + Sync + 'static`
- [ ] `Route`, `Router` e `Scope` usam `Arc<dyn Middleware<State>>`
- [ ] `cargo check -p gerax-http` passa sem erros nem warnings

## Dependências

Tarefa 1.1 (precisa de `ServerResult` e `Handler::call` retornando `Result`)

## Notas

- O `Next::call()` é um placeholder (`unreachable!`) que será sobrescrito pelo `Router` no momento da execução. Isso é intencional — o Router constrói a cadeia de `Next` encadeados dinamicamente.
- A mudança de `Middleware` de marker trait para chain of responsibility é uma breaking change para qualquer implementação existente de `Middleware`.
- O método `name()` é mantrado para logging e debugging, mas não é mais usado na execução da cadeia.