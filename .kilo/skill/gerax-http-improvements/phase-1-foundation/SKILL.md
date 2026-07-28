# Fase 1 — Fundação

## Objetivo

Estabelecer as bases para um sistema de handlers e middleware robusto, tornando possível a propagação e o tratamento estruturado de erros em toda a cadeia de requisições.

## Visão Geral

Atualmente, `Handler::call` retorna `Response` diretamente (sem possibilidade de erro), e `Middleware` é apenas um marker trait sem poder de interceptação. Esta fase transforma o sistema para suportar:

- Handlers que retornam `Result<Response, HttpServerError>`
- Middleware com poder de interceptação pré e pós-processamento
- Cadeia de execução de middlewares (chain of responsibility)
- Propagação de erros do middleware ao handler e vice-versa

## Ordem de Execução das Tarefas

```
1.1 → 1.2 → 1.3 → 1.4 → 1.5
```

## Dependências entre Tarefas

| Tarefa | Depende de |
|--------|-----------|
| 1.1 | Nenhuma |
| 1.2 | 1.1 |
| 1.3 | 1.1 |
| 1.4 | 1.1, 1.3 |
| 1.5 | 1.4 |

## Arquivos Afetados

| Arquivo | Tarefas |
|---------|---------|
| `crates/gerax-http/src/error.rs` | 1.1 |
| `crates/gerax-http/src/routing/handler.rs` | 1.1, 1.2 |
| `crates/gerax-http/src/middleware/mod.rs` | 1.3, 1.4 |
| `crates/gerax-http/src/routing/route.rs` | 1.4 |
| `crates/gerax-http/src/routing/router.rs` | 1.5 |
| `crates/gerax-http/src/routing/scope.rs` | 1.5 |
| `crates/gerax-http/src/routing/context.rs` | 1.3 |
| `crates/gerax-http/src/lib.rs` | 1.5 (opcional) |

## Fluxo de Execução

```
Request chega
  → Router::handle()
    → Route::execute()
      → Middleware[0].handle(ctx, Next)
        → Middleware[1].handle(ctx, Next)
          → ...
            → Handler::call(ctx)
              → Retorna ServerResult<Response>
            ← Handler retorna Result
          ← Middleware pós-processa Result
        ← Middleware retorna Result
      ← Route retorna Result
    ← Router retorna Result
  ← Server retorna Result
```

## Critérios de Conclusão da Fase

- [ ] Tarefa 1.1 completa
- [ ] Tarefa 1.2 completa
- [ ] Tarefa 1.3 completa
- [ ] Tarefa 1.4 completa
- [ ] Tarefa 1.5 completa
- [ ] `cargo check -p gerax-http` passa sem erros nem warnings
- [ ] Todos os handlers existentes retornam `ServerResult<Response>`
- [ ] Middleware pode interceptar e transformar requisições/respostas
- [ ] Erros em qualquer etapa são propagados corretamente