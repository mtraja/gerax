# gerax-postgres — Specification

## Contratos

Implementa `gerax-db::Repository<T>` para **PostgreSQL** via `tokio-postgres`.

- Conexão via `tokio_postgres::connect(uri, TcpStream).await`.
- Queries parametrizadas para evitar SQL injection.
- `id` pode ser `String` ou `Uuid`, dependendo do schema.

## Regras
- Depende de `gerax-core`, `tokio-postgres`, `futures`.
- Deve suportar `with-chrono-0_4` feature do `tokio-postgres`.
- Transações devem ser suportadas via `tokio_postgres::Transaction`.

## Testes Esperados
- Integration test com PostgreSQL via container Docker.
- Teste de migração (se houver sistema de migrations).
