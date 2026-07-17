# gerax-core — Specification

## Contratos

### `trait Entity`
- **Invariante**: Qualquer tipo que implemente `Entity` é serializável, cloneável e `Send + Sync + Unpin`.
- **`collection_name()`**: retorna o nome da coleção/tabela no banco. Deve ser estável e único por entidade.
- **`id(&self)`**: retorna `Option<String>`. Retorna `Some(id)` após persistência; `None` antes.
- **`set_id(&mut self, id: String)`**: atribui o identificador. Não valida formato; cabe à implementação garantir unicidade.

## Regras
- Nenhuma dependência de crates de framework ou banco de dados.
- Dependências mínimas: `serde`, `async-trait`, `thiserror`.

## Testes Esperados
- Unit test: entidade mock implementa `Entity` e satisfaz todos os bound traits.
- Property test: serialização + deserialização de uma entidade é reversível.
