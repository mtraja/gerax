---
name: gerax-db-repository
description: Implementa a trait Repository para a crate gerax-db seguindo o spec repository.md
---

# Skill: gerax-db-repository

Use quando precisar implementar ou modificar a trait `Repository<T>` na crate `gerax-db`.

## Especificação

Leia `crates/gerax-db/repository.md` e `crates/gerax-db/SPEC.md` antes de implementar.

## Contratos obrigatórios

Defina a trait `Repository<T>` onde `T: Entity + Send + Sync` com estes métodos assíncronos:

- `find_by_id(id: &str) -> Result<Option<T>, Error>`
- `find_all() -> Result<Vec<T>, Error>`
- `insert(entity: T) -> Result<T, Error>`
- `update(entity: T) -> Result<(), Error>`
- `delete(id: &str) -> Result<(), Error>`

## Regras

- Todos os métodos são async e usam `async-trait`.
- Erros usam `thiserror` e convertem para `gerax-core::Error`.
- A implementação é independente de banco de dados.
- Nenhuma crate de banco é exposta na fachada.

## Erros esperados

- ConnectionError
- SerializationError  
- NotFoundError

## Testes esperados

- Deve criar uma entidade.
- Deve buscar uma entidade existente.
- Deve retornar None quando não existir.
