# gerax-axum — Specification

## Contratos

Implementa `gerax-http::HttpServer` e `gerax-http::RouteProvider` para **Axum 0.8**.

- `AxumServer::new()`: constrói instância com configuração padrão.
- `run(state: AppState)`: inicia o servidor Axum em `0.0.0.0:8080` (configurável).
- `routes(state: AppState)`: mapeia rotas definidas pelo usuário para `axum::Router`.

## Regras
- Depende apenas de `gerax-core`, `gerax-http` e `axum`.
- Não deve vazar tipos de Axum na API pública do crate.
- Deve compilar com `axum = { version = "0.8", features = ["macros"] }`.

## Testes Esperados
- Teste de integração: sobe servidor em porta aleatória, faz request HTTP, valida status 200.
- Teste de rota protegida (se `auth` feature habilitada).
