# gerax-poem — Specification

## Contratos

Implementa `gerax-http::HttpServer` e `gerax-http::RouteProvider` para **Poem 3**.

- `PoemServer::new()`: constrói instância com configuração padrão.
- `run(state: AppState)`: inicia o servidor Poem em `0.0.0.0:8080` (configurável).
- `routes(state: AppState)`: mapeia rotas definidas pelo usuário para `poem::Route`.

## Regras
- Depende apenas de `gerax-core`, `gerax-http` e `poem`.
- Não deve vazar tipos de Poem na API pública do crate.
- Deve compilar com `poem = { version = "3" }`.

## Testes Esperados
- Teste de integração: sobe servidor em porta aleatória, faz request HTTP, valida status 200.
- Teste de rota protegida (se `auth` feature habilitada).
