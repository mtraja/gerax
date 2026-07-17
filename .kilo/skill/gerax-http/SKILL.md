---
name: gerax-http
description: Implementa módulo de abstração HTTP na crate gerax-http seguindo o spec SPEC.md
---

# Skill: gerax-http

Use quando precisar implementar ou modificar a camada de abstração HTTP na crate `gerax-http`.

## Especificação

Leia `crates/gerax-http/SPEC.md` antes de implementar.

## Contratos obrigatórios

- `HttpServer<S>` trait genérico com método async `start(&self, state: S) -> Result<(), HttpError>`
- `HttpError` enum com variante `StartError(String)` via `thiserror`
- `RouteConfigurator` builder para configuração encadeável de rotas
- `RouteConfig` trait com método async `apply(&self, config: &mut RouteConfigurator)` (no-op padrão)
- `MockHttpServer` para testes, registrando chamadas

## Regras

- Não deve conhecer detalhes de framework (Actix/Axum/Poem).
- Deve garantir dependências entre camadas e módulos.
- A codificação deve ser independente de tecnologia.
- A aplicação escolhe a tecnologia HTTP.
- Use padrões como builder e facade.
- Erros usam `thiserror`.
- Todos os métodos de trait são async via `async-trait`.
- Testes devem rodar sem dependências de framework HTTP.
- `start` deve receber um estado genérico `S` (ex: `AppState`), permitindo injeção de dependências.
- `S` deve ser `Send + Sync + 'static` quando necessário para runtimes async.

## Estrutura esperada

- `HttpError` em `src/lib.rs`
- `HttpServer<S>` trait genérico
- `RouteConfigurator` com `add_route`, `configure`, `into_inner`
- `RouteConfig` trait com implementação padrão vazia
- `MockHttpServer` com `started: Arc<AtomicBool>` e `routes: Arc<Mutex<Vec<String>>>`
- Testes em `#[cfg(test)] mod tests`

## Testes esperados

- Mock que registra chamadas de `start` com estado.
- Teste garantindo que a implementação padrão de `RouteConfig::apply` não altera o estado.
