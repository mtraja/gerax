# gerax-auth — Specification

## Contratos

Fornece traits e tipos para autenticação/autorização.

- `trait Authenticator`: valida credenciais e retorna Claims.
- `trait Authorizer`: verifica permissões sobre recursos.

## Regras
- Depende de `gerax-core`, `gerax-http`, `serde`, `async-trait`.
- JWT deve usar algoritmo seguro (HS256 ou RS256).
- Tokens de refresh devem ser rotacionáveis.

## Testes Esperados
- Teste de login/logout com credenciais válidas e inválidas.
- Teste de acesso negado a recurso protegido.
- Teste de expiração de token.
