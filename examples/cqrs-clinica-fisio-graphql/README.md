# cqrs-clinica-fisio-graphql

Aplicação exemplo com **arquitetura hexagonal** para uma clínica de
fisioterapia usando:

- [`gerax-cqrs`](../../crates/gerax-cqrs) — Command/Query Bus, handlers e registry
- [`gerax-postgres`](../../crates/gerax-postgres) — repositórios sobre PostgreSQL
- [`gerax-graphql`](../../crates/gerax-graphql) — schema e execução GraphQL
- **axum** — servidor HTTP (adaptador de entrada GraphQL)
- [`gerax-macros`](../../crates/gerax-macros) — derives `Command`, `Query` e `Entity`

Entidades do domínio: **Paciente** e **Terapia**.

## Estrutura

```
src/
├── main.rs                         # ponto de entrada
├── bootstrap.rs                    # composition root (conexão, registro, schema, router)
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
    ├── inbound/graphql/            # API GraphQL com axum + gerax-graphql
    │   ├── dto.rs                  # tipos GraphQL (SimpleObject/InputObject)
    │   ├── error.rs                # conversão de CqrsError para erro GraphQL
    │   ├── resolvers.rs            # QueryRoot e MutationRoot (resolvers)
    │   └── routes.rs               # router axum (/graphql e /graphiql)
    └── outbound/postgres/          # repositórios concretos com gerax-postgres
```

### Fluxo

```text
POST /graphql (axum)
   │  executa o Schema (gerax-graphql)
   ▼
QueryRoot / MutationRoot  (adapters/inbound/graphql/resolvers.rs)
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
   cargo run -p cqrs-clinica-fisio-graphql
   ```

4. Consuma a API em `http://localhost:8080/graphql` ou abra o GraphiQL
   em `http://localhost:8080/graphiql`.

### Queries

```graphql
query {
  pacientes { id nome cpf dataNascimento telefone email }
  paciente(id: "1") { id nome email }

  terapias { id tipo descricao sessaoSemanal qtdSessoes }
  terapia(id: "1") { id tipo }
  terapiasDoPaciente(pacienteId: "1") { id tipo descricao }
}
```

### Mutations

```graphql
mutation {
  criarPaciente(input: {
    nome: "Maria Silva"
    cpf: "12345678900"
    dataNascimento: "1990-05-04"
    telefone: "(85) 99999-0000"
    email: "maria@email.com"
  }) {
    id nome cpf
  }
}

mutation {
  atualizarPaciente(id: "1", input: {
    nome: "Maria Souza"
    telefone: "(85) 98888-1111"
    email: "m.souza@email.com"
  }) {
    id nome
  }
}

mutation {
  deletarPaciente(id: "1")
}

mutation {
  criarTerapia(input: {
    pacienteId: "1"
    tipo: "Fisioterapia Ortopédica"
    descricao: "Reabilitação de joelho"
    sessaoSemanal: 3
    qtdSessoes: 10
  }) {
    id tipo
  }
}

mutation {
  deletarTerapia(id: "1")
}
```

Ou com `curl`:

```bash
curl -s http://localhost:8080/graphql \
  -H 'content-type: application/json' \
  -d '{ "query": "{ pacientes { id nome } }" }'
```

## Erros

A camada `application` define `ApplicationError` (NotFound, BusinessRule,
Infrastructure). Ao atravessar o `gerax-cqrs`, os erros são convertidos
para `CqrsError::Execution` com uma tag prefixada (`[not_found]`,
`[business_rule]`, `[infrastructure]`); o adaptador GraphQL remove a tag
e expõe apenas a mensagem no campo `errors` da resposta.

## Testes

```bash
cargo test -p cqrs-clinica-fisio-graphql
```

Os testes usam repositórios fake em memória, sem banco — inclusive os
testes do adaptador GraphQL, que executam queries e mutations reais
contra o schema montado com as buses.