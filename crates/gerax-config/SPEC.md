# gerax-config — Specification

## Contratos

Carrega configuração de variáveis de ambiente e `.env`.

- `Config::from_env()`: lê variáveis prefixadas (ex: `GERAX_DATABASE_URL`).
- Validação via `serde` + `schemars` (opcional, para JSON Schema).

## Regras
- Depende de `serde` e `dotenv`.
- Nunca falha com `panic!` em produção; retorna `Result` com mensagem clara.
- Arquivo `.env` é carregado automaticamente em desenvolvimento.

## Testes Esperados
- Teste de carregamento de variáveis válidas.
- Teste de falha por variável obrigatória ausente.
- Teste de sobrescrita de `.env` por variável de ambiente real.
