# Repository

## Objetivo

Define a interface de persistência de entidades.

## Contexto

O Repository abstrai o mecanismo de armazenamento, permitindo que a aplicação funcione com MongoDB, PostgreSQL ou qualquer outro backend. A interface deve definir os metodos para atender os requisitos definidos.

## Requisitos

### REQ-001 - Criar entidade
### REQ-002 - Buscar entidade por ID  
### REQ-003 - Listar todas as instancia de uma entidade
### REQ-004 - Atualizar entidade dado um ID
### REQ-005 - Deletar entidade


## Casos de erro

- Erro de conexão.
- Erro de serialização.
- Erro de ID não existente.


## Critérios de aceitação

- Todos os métodos são assíncronos.
- A implementação é independente do banco.

## Testes esperados

- Deve criar uma entidade.
- Deve buscar uma entidade existente.
- Deve retornar None quando não existir.


