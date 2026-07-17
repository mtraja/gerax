# gerax-actix — Specification

## Contratos

Implementa as abstrações de `gerax-http` para o framework **Actix Web**.

- Um servidor HTTP recebe um estado compartilhado no momento de inicialização.
- O método de inicialização inicia o servidor e bloqueia até encerramento ou erro.
- As rotas são construídas a partir do estado compartilhado.
- A aplicação de configuração de rotas é realizada através de encadeamento.
- A inicialização padrão não altera o estado quando nenhuma rota é configurada.

## Regras

- Depende apenas de `gerax-core`, `gerax-http` e `actix-web`.
- Não deve vazar tipos específicos do Actix na API pública do crate.
- A inicialização deve ocorrer em porta configurável, com padrão em `0.0.0.0:8080`.
- O estado compartilhado deve ser seguro para uso concorrente.
- Erros são representados pela hierarquia definida em `gerax-http`.
- O método de inicialização é assíncrono.

## Testes Esperados

- Teste de integração: sobe o servidor em porta aleatória, faz requisição HTTP e valida status 200.
- Teste de rota protegida (se recurso de autenticação estiver habilitado).
- Teste garantindo que a inicialização padrão não altera o estado quando nenhuma rota é configurada.
