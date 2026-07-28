# Tarefa 1.4: Atualizar `Route` para usar `Handler<State>` com `Result` e `Next`

## Fase
Fase 1 — Fundação

## Objetivo
Garantir que `Route` armazene handlers que retornam `ServerResult<Response>` e execute a cadeia de middleware corretamente usando `Next`. Adicionar o método `Route::execute()` que orquestra a execução de middlewares e handler.

## Arquivos Afetados
- `crates/gerax-http/src/routing/route.rs`

## Instruções Detalhadas

### Passo 1: Atualizar a struct `Route`

Em `crates/gerax-http/src/routing/route.rs`, atualize o tipo de `middlewares` para usar `Arc<dyn Middleware<State>>`:

```rust
use super::{Handler, HttpMethod};
use crate::Middleware;
use std::sync::Arc;

pub struct Route<State> {
    method: HttpMethod,
    path: String,
    handler: Arc<dyn Handler<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}
```

### Passo 2: Atualizar `Route::new`

```rust
impl<State> Route<State> {
    pub fn new<H>(method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        Self {
            method,
            path: path.into(),
            handler: Arc::new(handler),
            middlewares: Vec::new(),
        }
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn handler(&self) -> &Arc<dyn Handler<State>> {
        &self.handler
    }

    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>] {
        &self.middlewares
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<State>,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }
}
```

### Passo 3: Implementar `Route::execute()`

Adicione o método `execute` que orquestra a cadeia de middleware e o handler:

```rust
impl<State> Route<State> {
    pub async fn execute(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        if let Some(middleware) = self.middlewares.first() {
            let remaining = self.middlewares.iter().skip(1).cloned().collect();
            let next = Next::new_with_chain(ctx, remaining, Arc::clone(&self.handler));
            middleware.handle(ctx, next).await
        } else {
            self.handler.call(ctx).await
        }
    }
}
```

### Passo 4: Implementar `Next` com encadeamento completo

Em `crates/gerax-http/src/middleware/mod.rs`, atualize `Next` para suportar encadeamento de middlewares restantes + handler final:

```rust
use crate::{Context, ServerResult, Response};
use std::sync::Arc;

pub struct Next<State> {
    state: Context<State>,
    remaining_middlewares: Vec<Arc<dyn Middleware<State>>>,
    handler: Arc<dyn Handler<State>>,
}

impl<State> Next<State> {
    pub fn new(context: Context<State>) -> Self {
        Self {
            state: context,
            remaining_middlewares: Vec::new(),
            handler: Arc::new(NoOpHandler),
        }
    }

    pub fn new_with_chain(
        context: Context<State>,
        remaining_middlewares: Vec<Arc<dyn Middleware<State>>>,
        handler: Arc<dyn Handler<State>>,
    ) -> Self {
        Self {
            state: context,
            remaining_middlewares,
            handler,
        }
    }

    pub async fn call(self) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        if let Some(middleware) = self.remaining_middlewares.first() {
            let next_remaining = self.remaining_middlewares.iter().skip(1).cloned().collect();
            let next = Next::new_with_chain(self.state, next_remaining, self.handler);
            middleware.handle(self.state, next).await
        } else {
            self.handler.call(self.state).await
        }
    }
}

struct NoOpHandler;

#[async_trait]
impl<State> Handler<State> for NoOpHandler
where
    State: Send + Sync + 'static,
{
    async fn call(&self, _ctx: Context<State>) -> ServerResult<Response> {
        Err(HttpServerError::HandlerError("No handler".to_string()))
    }
}
```

### Passo 5: Verificar compilação

Execute `cargo check -p gerax-http` para garantir que todas as mudanças compilam sem erros nem warnings.

## Código de Referência

### `route.rs` — estado final esperado

```rust
use super::{Handler, HttpMethod};
use crate::Middleware;
use std::sync::Arc;

pub struct Route<State> {
    method: HttpMethod,
    path: String,
    handler: Arc<dyn Handler<State>>,
    middlewares: Vec<Arc<dyn Middleware<State>>>,
}

impl<State> Route<State> {
    pub fn new<H>(method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>,
    {
        Self {
            method,
            path: path.into(),
            handler: Arc::new(handler),
            middlewares: Vec::new(),
        }
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn handler(&self) -> &Arc<dyn Handler<State>> {
        &self.handler
    }

    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>] {
        &self.middlewares
    }

    pub fn middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<State>,
    {
        self.middlewares.push(Arc::new(middleware));
        self
    }

    pub async fn execute(&self, ctx: Context<State>) -> ServerResult<Response>
    where
        State: Send + Sync + 'static,
    {
        if let Some(middleware) = self.middlewares.first() {
            let remaining = self.middlewares.iter().skip(1).cloned().collect();
            let next = Next::new_with_chain(ctx, remaining, Arc::clone(&self.handler));
            middleware.handle(ctx, next).await
        } else {
            self.handler.call(ctx).await
        }
    }
}
```

## Critérios de Aceite

- [ ] `Route` armazena `middlewares: Vec<Arc<dyn Middleware<State>>>`
- [ ] `Route::execute()` executa a cadeia de middleware + handler
- [ ] Erro em qualquer middleware interrompe a cadeia e retorna o erro
- [ ] Sem middlewares, `execute()` chama o handler diretamente
- [ ] `cargo check -p gerax-http` passa sem erros nem warnings

## Dependências

- Tarefa 1.1 (`Handler::call` retorna `ServerResult<Response>`)
- Tarefa 1.3 (`Middleware` trait com `handle` e `Next`)

## Notas

- `Next` é o elo encadeado da cadeia de middleware. Cada middleware recebe um `Next` que sabe quais middlewares restam executar e qual é o handler final.
- O `NoOpHandler` é um fallback que retorna erro caso `Next::call()` seja invocado sem um handler válido. Isso nunca deve acontecer em uso normal.
- A ordem de execução dos middlewares é FIFO: o primeiro middleware adicionado é o primeiro a executar (pré-processamento), e o último a receber o controle no pós-processamento.