# RepositoryPostgre

## Objetivo

Implementar da interface de persistência de entidades para banco de dados PostgreSQL

## Contexto

O RepositoryPostgre deve implementar a interface de persistência empregando todos os recursos disponiveis nos SGBD. A implementação concreta deve prever todos metodos  da interface Repository para atender os requisitos definidos.

## Requisitos

- A conexao deve atender aos parametros do SGBD
- Os parametros de autenticao deve ser definidos em arquivos para garante seguranca
- O nome do bando de dados sera definido em arquivo

## Casos de erro

- Erro de conexão.
- Erro de serialização.
- Erro de ID não existente.


## Critérios de aceitação

- Todos os métodos são assíncronos.
- Dependencias entre modulos atendidas

## Testes esperados

- Deve criar instancia da estrutura RepositoryPostgres
- Deve fazer conexão
- Deve criar uma entidade.
- Deve buscar uma entidade existente.
- Deve retornar None quando não existir.