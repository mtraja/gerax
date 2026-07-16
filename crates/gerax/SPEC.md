# gerax — Specification

## Contratos

### Crate Fachada
- Re-exporta tipos públicos de `gerax-core`, `gerax-macros` e crates opcionais conforme features habilitadas.
- Features padrão: `actix`, `mongodb`.
- Nenhuma lógica de negócio deve residir na fachada; apenas re-exports e feature gating.

### Features
- `actix`: habilita `gerax-http` + `gerax-actix`.
- `axum`: habilita `gerax-http` + `gerax-axum`.
- `poem`: habilita `gerax-http` + `gerax-poem`.
- `mongodb`: habilita `gerax-mongodb`.
- `postgres`: habilita `gerax-postgres`.
- `auth`: habilita `gerax-auth`.
- `config`: habilita `gerax-config`.

## Regras
- Não deve depender diretamente de frameworks ou bancos de dados.
- Todas as dependências de banco e framework devem ser opcionais via features.
- Manter `lib.rs` limpo: apenas `pub use` e `#[cfg(feature = "...")]`.

## Testes Esperados
- Doc test com exemplo canônico.
- Verificação de re-exports por feature (teste de compilação condicional).
