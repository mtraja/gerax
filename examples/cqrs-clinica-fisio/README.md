# cqrs-clinica-fisio

Aplicação exemplo com **arquitetura hexagonal** para uma clínica de
fisioterapia usando:

- [`gerax-cqrs`](../../crates/gerax-cqrs) — Command/Query Bus, handlers e registry
- [`gerax-postgres`](../../crates/gerax-postgres) — repositórios sobre PostgreSQL
- **axum** — API REST (adaptador de entrada HTTP)
- [`gerax-macros`](../../crates/gerax-macros) — derives `Command`, `Query` e `Entity`

Entidades do domínio: **Paciente** e **Terapia**.

## Estrutura

```
src/
├── main.rs                         # ponto de entrada
├── bootstrap.rs                    # composition root (conexão, registro, buses, router)
├── domain/                         # entidades e regras de negócio (sem infraestrutura)
│   ├── paciente.rs
│   ├── terapia.rs
│   └── error.rs
├── application/                    # núcleo da aplicação (gerax-cqrs vive aqui)
│   ├── errors.rs                   # ApplicationError (erro da aplicação)
│   ├── messages/                   # Commands e Queries (mensagens CQRS)
│   ├── ports/outbound/             # contratos de persistência (repositórios)
│   └── handlers/                   # implementações de CommandHandler/QueryHandler
└── adapters/                       # infraestrutura (depende do núcleo, e não o contrário)
    ├── inbound/http/               # API REST com axum
    └── outbound/postgres/          # repositórios concretos com gerax-postgres
```

### Fluxo

```text
HTTP (axum)
   │  dispatch / execute
   ▼
CommandBus / QueryBus  (gerax-cqrs)
   │  resolve handler pelo TypeId
   ▼
CommandHandler / QueryHandler  (application/handlers)
   │  usa a porta
   ▼
PacienteRepository / TerapiaRepository  (ports/outbound)
   │  implementada por
   ▼
Postgres*Repository  (adapters/outbound/postgres)
```

## Como rodar

1. Suba o PostgreSQL:

   ```bash
   docker compose -f compose.yaml up -d
   ```

2. Configure a conexão. O exemplo lê um arquivo `.env` na raiz do
   exemplo (o `.env` não é versionado — `.env` está no `.gitignore` do
   workspace). Exemplo:

   ```env
   DATABASE_URL=postgresql://gerax:gerax@localhost:5432/clinica_fisio
   ```

3. Rode a aplicação:

   ```bash
   cargo run -p cqrs-clinica-fisio
   ```

4. Consuma a API em `http://localhost:8080`:

   | Método | Rota                         | Ação                        |
   |--------|------------------------------|-----------------------------|
   | GET    | `/pacientes`                 | lista pacientes             |
   | POST   | `/pacientes`                 | cria paciente               |
   | GET    | `/pacientes/:id`             | obtém paciente              |
   | PUT    | `/pacientes/:id`             | atualiza paciente           |
   | DELETE | `/pacientes/:id`             | remove paciente             |
   | GET    | `/pacientes/:id/terapias`    | lista terapias do paciente  |
   | GET    | `/terapias`                  | lista terapias              |
   | POST   | `/terapias`                  | cria terapia                |
   | GET    | `/terapias/:id`              | obtém terapia               |
   | PUT    | `/terapias/:id`              | atualiza terapia            |
   | DELETE | `/terapias/:id`              | remove terapia              |

   Exemplos `curl`:

   ```bash
   curl -s http://localhost:8080/pacientes \
     -H 'content-type: application/json' \
     -d '{"nome":"Maria Silva","cpf":"12345678900","data_nascimento":"1990-05-04","telefone":"(85) 99999-0000","email":"maria@email.com"}'

   curl -s http://localhost:8080/pacientes

   curl -s http://localhost:8080/terapias \
     -H 'content-type: application/json' \
     -d '{"paciente_id":"<id>","tipo":"Fisioterapia Ortopédica","descricao":"Reabilitação de joelho","sessao_semanal":3,"qtd_sessoes":10}'
   ```

## Erros

A camada `application` define `ApplicationError` (NotFound, BusinessRule,
Infrastructure). Ao atravessar o `gerax-cqrs`, os erros são convertidos
para `CqrsError::Execution` com uma tag prefixada (`[not_found]`,
`[business_rule]`, `[infrastructure]`); o adaptador HTTP traduz a tag
de volta para o status HTTP (404, 400 ou 500).

## Testes

```bash
cargo test -p cqrs-clinica-fisio
```

Os testes usam repositórios fake em memória, sem banco.