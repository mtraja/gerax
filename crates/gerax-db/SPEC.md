# gerax-db — Specification

## Contratos

### `trait Repository<T>`
*(a ser definido)*
- `find_by_id(id: &str) -> Result<Option<T>, Error>`
- `find_all() -> Result<Vec<T>, Error>`
- `insert(entity: T) -> Result<T, Error>` — retorna entidade com `id` preenchido.
- `update(entity: T) -> Result<(), Error>`
- `delete(id: &str) -> Result<(), Error>`

### `trait Connection`
*(a ser definido)*
- `fn connect() -> Result<Self, Error> where Self: Sized;`
- `fn ping(&self) -> Result<(), Error>;`

## Regras
- Operações são async e retornam `Future<Output = Result<_, _>>`.
- Erros devem ser convertidos para `gerax-core::Error` ou tipados localmente via `thiserror`.
- Nenhuma crate de banco deve ser exposta na fachada `gerax` diretamente.

## Testes Esperados
- Mock repository para testes unitários de serviço.
- Integration test com banco em memória ou container (se aplicável).
