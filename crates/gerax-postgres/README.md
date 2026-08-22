# gerax-postgres

Adaptador PostgreSQL para o framework Gerax. Ele fornece uma conexão assíncrona
baseada em `tokio-postgres` e um repositório genérico que implementa os traits
`Connection` e `Repository<T>` de `gerax-db`.

Cada entidade é armazenada em uma tabela com duas colunas: `id` (`TEXT`, chave
primária) e `data` (`TEXT`, com a entidade serializada em JSON).

## Dependência

Em um projeto dentro deste workspace:

```toml
[dependencies]
gerax-postgres = { path = "../crates/gerax-postgres" }
gerax-core = { path = "../crates/gerax-core" }
gerax-db = { path = "../crates/gerax-db" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Configuração

`PostgresConnection::connect()` lê variáveis do processo e, opcionalmente, o
arquivo `.env` por meio de `gerax-config`. A ordem de carregamento é:

1. `.env`
2. variáveis do processo

A última fonte carregada tem precedência. A URL da conexão pode ser fornecida
por qualquer uma das chaves abaixo (normalizadas para minúsculas):

```dotenv
URL=postgresql://postgres:postgres@localhost:5432/gerax
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/gerax
POSTGRES_URL=postgresql://postgres:postgres@localhost:5432/gerax
```

### TLS

TLS é uma decisão de configuração explícita. O padrão é **desativado**; para
habilitá-lo, defina a variável `tls` como `true` (ou `1`, `enable`, `enabled`):

```dotenv
URL=postgresql://postgres:postgres@localhost:5432/gerax
TLS=true
```

Quando TLS está habilitado, a conexão usa `native-tls` via `postgres-native-tls`.
Para conexões sem TLS, mantenha `TLS=false` ou omita a variável.

Também é possível configurar a conexão de forma programática com
`PostgresConfig`:

```rust
use gerax_postgres::{PostgresConfig, PostgresTls};

let config = PostgresConfig::new("postgresql://localhost/gerax")
    .with_tls(PostgresTls::Enabled);
```

### Validação

Se nenhuma das chaves de URL estiver presente, ou se a URL for vazia, a conexão
retorna `DbError::ConfigurationError` antes de tentar abrir o soquete.

## Uso

Implemente `Entity` para o tipo que será persistido. O valor retornado por
`collection_name()` é usado diretamente como nome da tabela; portanto, use um
nome constante e confiável.

```rust
use std::sync::Arc;

use gerax_core::Entity;
use gerax_db::{Connection, Repository};
use gerax_postgres::{PostgresConnection, PostgresRepository};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
struct User {
    id: Option<String>,
    name: String,
}

impl Entity for User {
    fn collection_name() -> &'static str {
        "users"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Arc::new(PostgresConnection::connect().await?);
    connection.ping().await?;

    let repository = PostgresRepository::<User>::new(connection);
    repository.create_table().await?;

    let created = repository
        .insert(User {
            id: None,
            name: "Ada".into(),
        })
        .await?;

    let user = repository
        .find_by_id(&created.id().expect("insert atribui um id"))
        .await?;
    assert!(user.is_some());

    Ok(())
}
```

## API pública

- `PostgresConnection`: implementa `Connection`. Use `connect()` para criar a
  conexão e `ping()` para verificar sua disponibilidade. `client()` expõe o
  `tokio_postgres::Client` subjacente para operações específicas do PostgreSQL.
- `PostgresConfig`: configuração explícita com `url` e `tls`. Use
  `connect_with_config(config)` para evitar dependência de variáveis de ambiente.
- `PostgresConnection::try_connection_error()`: consulta não bloqueante para
  verificar se a task do driver reportou uma falha de conexão, TLS ou rede.
- `PostgresRepository<T>`: recebe um `Arc<PostgresConnection>` em `new()` e
  implementa `Repository<T>`. Também oferece `create_table()` e `connection()`.
- `PostgresRepositoryBuilder<T>`: recebe um `DatabaseConfig` em `new()` e pode
  receber uma conexão já aberta com `with_connection()`. `build()` retorna
  `Box<dyn Repository<T>>`; sem uma conexão explícita, ele chama
  `PostgresConnection::connect()`.

`PostgresRepository<T>` exige que `T` implemente `Entity`, `Serialize` e
`Deserialize` (além de `Send`, `Sync` e `'static`).

## Operações do repositório

| Método | Comportamento |
| --- | --- |
| `find_by_id(id)` | Retorna `Ok(Some(entity))` ou `Ok(None)` quando não há registro. |
| `find_all()` | Retorna todas as entidades da tabela. |
| `insert(entity)` | Gera um ID baseado no tempo quando a entidade não possui um e retorna a entidade gravada. |
| `update(entity)` | Atualiza os dados JSON; retorna `NotFoundError` se o ID estiver ausente ou não existir. |
| `delete(id)` | Remove o registro; retorna `NotFoundError` se ele não existir. |

Todos os métodos retornam `Result<_, gerax_db::DbError>`. Falhas de conexão e
SQL são mapeadas para `ConnectionError`; falhas na conversão JSON são mapeadas
para `SerializationError`. Problemas de configuração são mapeados para
`ConfigurationError`.

## Limitações

`create_table()` cria somente a estrutura mínima `id`/`data` e não substitui
migrações. O crate não faz escape ou validação do nome retornado por
`collection_name()`; esse valor não deve vir de entrada externa. A task do
driver é executada em segundo plano enquanto o `Client` estiver em uso; falhas
são capturadas por `try_connection_error()`, mas não há reconexão automática.
