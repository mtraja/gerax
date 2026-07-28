# Tarefa 1.2: Atualizar `Handler` blanket impl para `Fn` retornar `Result`

## Fase
Fase 1 — Fundação

## Objetivo
Garantir que funções que retornam `Response` diretamente gerem erro de tipo, forçando todos os handlers a retornarem `ServerResult<Response>`. Esta tarefa é uma verificação e refinamento da Tarefa 1.1.

## Arquivos Afetados
- `crates/gerax-http/src/routing/handler.rs`

## Instruções Detalhadas

### Passo 1: Verificar o estado atual do blanket impl

Abra `crates/gerax-http/src/routing/handler.rs` e confirme que o blanket impl exige `Fut: Future<Output = ServerResult<Response>>`. Se a Tarefa 1.1 foi implementada corretamente, isso já estará feito.

### Passo 2: Testar que `Response` direto não compila

Crie um arquivo de teste temporário ou adicione um teste de compilação que tente usar uma função que retorna `Response` diretamente como handler:

```rust
// Este handler NÃO deve compilar:
async fn bad_handler(_ctx: Context<()>) -> Response {
    Response::ok("test")
}

// Tentativa de registro — deve gerar erro de tipo:
// router.get("/bad", bad_handler);
// Error: expected `ServerResult<Response>`, found `Response`
```

### Passo 3: Testar que `ServerResult<Response>` compila

```rust
// Este handler DEVE compilar:
async fn good_handler(_ctx: Context<()>) -> ServerResult<Response> {
    Ok(Response::ok("test"))
}

// Registro — deve funcionar:
// router.get("/good", good_handler);
```

### Passo 4: Atualizar handlers existentes no códigobase

Se houver handlers existentes no códigobase (em `gerax-actix`, `gerax-http`, etc.) que retornam `Response` diretamente, atualize-os para retornar `ServerResult<Response>`:

```rust
// Antes:
async fn my_handler(ctx: Context<()>) -> Response {
    Response::ok("hello")
}

// Depois:
async fn my_handler(ctx: Context<()>) -> ServerResult<Response> {
    Ok(Response::ok("hello"))
}
```

### Passo 5: Verificar compilação global

Execute `cargo check -p gerax-http` e `cargo check -p gerax-actix` (ou qualquer crate que use `gerax-http`) para garantir que todos os handlers existentes foram atualizados.

## Código de Referência

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

- [ ] Handler que retorna `Response` diretamente não compila (erro de tipo)
- [ ] Handler que retorna `ServerResult<Response>` compila normalmente
- [ ] Todos os handlers existentes no códbase retornam `ServerResult<Response>`
- [ ] `cargo check -p gerax-http` passa sem erros nem warnings
- [ ] `cargo check -p gerax-actix` passa sem erros nem warnings

## Dependências

Tarefa 1.1

## Notas

- Esta tarefa é uma verificação da Tarefa 1.1. Se o blanket impl já está correto na 1.1, esta tarefa serve como validação.
- A mudança de `Response` para `ServerResult<Response>` é uma quebra de compatibilidade (breaking change). Todos os handlers existentes precisarão ser envolvidos em `Ok(...)`.
- Se a crate `gerax-actix` ou qualquer outra crate do workspace usa handlers que retornam `Response` diretamente, eles precisarão ser atualizados também.