# Tarefa 1.1: Criar `HandlerError` e atualizar `Handler::call` para `Result`

## Fase
Fase 1 — Fundação

## Objetivo
Adicionar a variante `HandlerError` ao enum `HttpServerError` e alterar a trait `Handler` para que seu método `call` retorne `Result<Response, HttpServerError>` em vez de `Response` direto. Isso permite que handlers propaguem erros de forma estruturada.

## Arquivos Afetados
- `crates/gerax-http/src/error.rs`
- `crates/gerax-http/src/routing/handler.rs`

## Instruções Detalhadas

### Passo 1: Adicionar `HandlerError` ao enum `HttpServerError`

Em `crates/gerax-http/src/error.rs`, adicione a nova variante ao enum:

```rust
#[derive(Debug, Error)]
pub enum HttpServerError {
    #[error("Falha ao iniciar o servidor: {0}")]
    InitializationFailed(String),

    #[error("Erro durante a execução do servidor: {0}")]
    RuntimeError(String),

    #[error("Erro de configuração: {0}")]
    ConfigurationError(String),

    #[error("Erro no handler: {0}")]
    HandlerError(String),
}
```

### Passo 2: Alterar `Handler::call` para retornar `ServerResult<Response>`

Em `crates/gerax-http/src/routing/handler.rs`, altere a trait `Handler`:

```rust
use async_trait::async_trait;

use super::{Context, Response};
use crate::ServerResult;

#[async_trait]
pub trait Handler<State>: Send + Sync + 'static {
    async fn call(&self, context: Context<State>) -> ServerResult<Response>;
}
```

### Passo 3: Atualizar o blanket impl para `Fn`

No mesmo arquivo, atualize o `impl` genérico para funções:

```rust
#[async_trait]
impl<State, F, Fut> Handler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(Context<State>) -> Fut,
    Fut: std::future::Future<Output = ServerResult<Response>> + Send,
{
    async fn call(&self, context: Context<State>) -> ServerResult<Response> {
        (self)(context).await
    }
}
```

### Passo 4: Verificar se `ServerResult` está importado corretamente

Em `error.rs`, confirme que `ServerResult` está definido como:

```rust
pub type ServerResult<T = ()> = Result<T, HttpServerError>;
```

Como `Response` é um tipo concreto, `ServerResult<Response>` resolve para `Result<Response, HttpServerError>`.

## Código de Referência

### `error.rs` — estado final esperado

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpServerError {
    #[error("Falha ao iniciar o servidor: {0}")]
    InitializationFailed(String),

    #[error("Erro durante a execução do servidor: {0}")]
    RuntimeError(String),

    #[error("Erro de configuração: {0}")]
    ConfigurationError(String),

    #[error("Erro no handler: {0}")]
    HandlerError(String),
}

pub type ServerResult<T = ()> = Result<T, HttpServerError>;
```

### `handler.rs` — estado final esperado

```rust
use async_trait::async_trait;

use super::{Context, Response};
use crate::ServerResult;

#[async_trait]
pub trait Handler<State>: Send + Sync + 'static {
    async fn call(&self, context: Context<State>) -> ServerResult<Response>;
}

#[async_trait]
impl<State, F, Fut> Handler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(Context<State>) -> Fut,
    Fut: std::future::Future<Output = ServerResult<Response>> + Send,
{
    async fn call(&self, context: Context<State>) -> ServerResult<Response> {
        (self)(context).await
    }
}
```

## Critérios de Aceite

- [ ] `HttpServerError` tem a variante `HandlerError(String)`
- [ ] `Handler::call` retorna `ServerResult<Response>`
- [ ] O blanket impl para `Fn` exige que a futura retorne `ServerResult<Response>` (não `Response`)
- [ ] `cargo check -p gerax-http` passa sem erros nem warnings
- [ ] `cargo test -p gerax-http` passa (se houver testes existentes)

## Dependências

Nenhuma — esta é a primeira tarefa da Fase 1.

## Notas

- Após esta tarefa, qualquer handler que anteriormente retornasse `Response` diretamente **não compilará mais**. Todos os handlers existentes precisarão ser atualizados para retornar `ServerResult<Response>` (envolvendo o `Response` em `Ok(...)`).
- A variante `HandlerError` é usada para erros específicos de handlers. Erros de middleware e do servidor em si continuam usando `InitializationFailed`, `RuntimeError` e `ConfigurationError`.
- A mudança de `Response` para `ServerResult<Response>` é a base para todas as tarefas subsequentes da Fase 1 (middleware chain, error propagation no router).