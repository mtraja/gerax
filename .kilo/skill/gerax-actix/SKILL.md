---
name: gerax-actix
description: Implementa módulo de integração com Actix Web na crate gerax-actix seguindo o spec SPEC.md
---

# Skill: gerax-actix

Use quando precisar implementar ou modificar a integração com Actix Web na crate `gerax-actix`.

## Especificação

Leia `crates/gerax-actix/SPEC.md` antes de implementar.

## Contratos obrigatórios

- Implementa as abstrações de `gerax-http` para o framework **Actix Web**.
- `ActixServer::new()`: constrói instância com configuração padrão.
- `run(state: AppState)`: inicia o servidor Actix Web em `0.0.0.0:8080` (configurável).
- `routes(state: AppState)`: mapeia rotas definidas pelo usuário para `actix_web::App`.
- Implementa `gerax_http::HttpServer<S>` para Actix Web.

## Regras

- Depende apenas de `gerax-core`, `gerax-http` e `actix-web`.
- Não deve vazar tipos específicos do Actix na API pública do crate.
- A inicialização deve ocorrer em porta configurável, com padrão em `0.0.0.0:8080`.
- O estado compartilhado deve ser seguro para uso concorrente (`Send + Sync + 'static`).
- Erros são representados pela hierarquia definida em `gerax-http::HttpError`.
- O método de inicialização é assíncrono.
- Usa `actix_web::HttpServer::bind` ou `actix_web::HttpServer::bind` com configuração de workers.

## Estrutura esperada

- `ActixServer` struct com configuração de porta e número de workers.
- Implementação de `gerax_http::HttpServer<S>` para `ActixServer`.
- Método `new()` para construção padrão.
- Método `run(state)` que inicia o servidor e bloqueia até encerramento.
- Método ou builder para configuração de rotas a partir do estado compartilhado.

## Testes esperados

- Teste de integração: sobe o servidor em porta aleatória, faz requisição HTTP e valida status 200.
- Teste de rota protegida (se recurso de autenticação estiver habilitado).
- Teste garantindo que a inicialização padrão não altera o estado quando nenhuma rota é configurada.
