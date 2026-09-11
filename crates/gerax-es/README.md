# gerax-es

Infraestrutura genérica e extensível de **Event Sourcing** para o ecossistema
Gerax. A crate expõe as portas (*traits*) do padrão, uma implementação em
memória para testes/protótipos e adapters opcionais de persistência
(PostgreSQL e MongoDB).

## Características

- `Aggregate` raiz com eventos pendentes — domínio **síncrono**, sem IO.
- `EventStore` como **porta** com *optimistic concurrency* no `append`.
- `AggregateRepository` genérico (`EventSourcedRepository`) que reidrata e
  persiste Aggregates via `store + serializer`.
- `EventSerializer` plugável; `JsonEventSerializer` incluído (eventos com tag
  interna `#[serde(tag = "event_type")]`).
- `EventMetadata` extensível (correlation/causation IDs no futuro).
- `SnapshotStore`/`Snapshot` opcionais para reidratação rápida.
- `EventPublisher` para publicar eventos **após** a persistência confirmada.
- Erros tipados com `thiserror` e conversão automática entre camadas.
- Adapters opcionais: `PostgresEventStore` (`postgres`) e `MongoDbEventStore`
  (`mongodb`).

## Visão geral do fluxo

```text
Domain (Aggregate)
  │  raise events (síncrono, sem IO)
  ▼
AggregateRepository        crate::repository
  │  load / save
  ▼
EventStore                 crate::event_store —— porta (adapters)
  │  append (optimistic concurrency)
  ▼
EventPublisher             crate::publisher —— porta
  │
  ▼
Consumers / Projections
```

## Módulos

| Módulo | Responsabilidade |
| --- | --- |
| `aggregate` | Trait `Aggregate`: estado, versão e eventos pendentes. |
| `version` | Posição do Aggregate no stream (`Version`), base do optimistic concurrency. |
| `event` | `EventId` e `StoredEvent` (evento persistido). |
| `metadata` | `EventMetadata` extensível por chave/valor. |
| `event_store` | Porta `EventStore`: `load` e `append`. |
| `memory` | `InMemoryEventStore` para testes e protótipos. |
| `repository` | `AggregateRepository` e `EventSourcedRepository` genérico. |
| `serialization` | `EventSerializer`, `JsonEventSerializer` e `SerializedEvent`. |
| `snapshot` | `Snapshot` e porta `SnapshotStore`. |
| `publisher` | Porta `EventPublisher` (publicação pós-persistência). |
| `error` | Erros: `ConcurrencyError`, `EventStoreError`, `RepositoryError`, etc. |
| `adapters` | `PostgresEventStore` e `MongoDbEventStore` (features opcionais). |

## Exemplo mínimo

```rust,no_run
use std::sync::Arc;

use gerax_es::{
    Aggregate, AggregateRepository, EventSourcedRepository, InMemoryEventStore,
    JsonEventSerializer, Version,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type")]
enum StudentEvent {
    StudentCreated { name: String },
    StudentRenamed { name: String },
}

#[derive(Debug)]
struct Student {
    id: String,
    name: String,
    version: Version,
    pending: Vec<StudentEvent>,
}

impl Aggregate for Student {
    type Id = String;
    type Event = StudentEvent;
    type Error = std::convert::Infallible;

    fn aggregate_type() -> &'static str {
        "Student"
    }

    fn empty(id: Self::Id) -> Self {
        Self {
            id,
            name: String::new(),
            version: Version::initial(),
            pending: Vec::new(),
        }
    }

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn version(&self) -> Version {
        self.version
    }

    fn apply(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        match event {
            StudentEvent::StudentCreated { name } | StudentEvent::StudentRenamed { name } => {
                self.name = name.clone();
            }
        }
        self.version = self.version.next();
        Ok(())
    }

    fn raise(&mut self, event: Self::Event) -> Result<(), Self::Error> {
        self.apply(&event)?;
        self.pending.push(event);
        Ok(())
    }

    fn take_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.pending)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(InMemoryEventStore::new());
    let repo = EventSourcedRepository::new(
        store.clone(),
        JsonEventSerializer::<StudentEvent>::new(),
    );

    let mut student = Student::empty("student-1".into());
    student.raise(StudentEvent::StudentCreated { name: "Ana".into() })?;
    repo.save(&mut student).await?;

    let reloaded = repo.load(&"student-1".to_string()).await?;
    println!("name={} version={}", reloaded.name, reloaded.version());
    Ok(())
}
```

## Exemplo completo

Veja [`examples/student.rs`](examples/student.rs) (Student Aggregate):

```text
CreateStudent → StudentCreated → persist
Load Student  → rehydrate
Rename        → StudentRenamed → persist
```

```sh
cargo run -p gerax-es --example student
```

## Adapters de persistência (opcionais)

Por padrão `gerax-es` vem apenas com `InMemoryEventStore`. Para persistência
real:

```text
gerax-es
  │
  ▼
EventStore (porta)
  │
  ├── InMemoryEventStore        (sempre disponível)
  ├── PostgresEventStore        (feature "postgres")
  └── MongoDbEventStore         (feature "mongodb")
```

### PostgreSQL

```sh
cargo add gerax-es --features postgres
```

```rust,no_run
use gerax_es::adapters::postgres::PostgresEventStore;
use gerax_es::{EventSourcedRepository, JsonEventSerializer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = PostgresEventStore::connect("postgres://user:pass@localhost/gerax").await?;
    store.ensure_schema().await?;
    let repo = EventSourcedRepository::new(store, JsonEventSerializer::new());
    Ok(())
}
```

### MongoDB

```sh
cargo add gerax-es --features mongodb
```

```rust,no_run
use gerax_es::adapters::mongodb::MongoDbEventStore;
use gerax_es::{EventSourcedRepository, JsonEventSerializer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = MongoDbEventStore::connect("mongodb://localhost:27017", "gerax").await?;
    store.ensure_schema().await?;
    let repo = EventSourcedRepository::new(store, JsonEventSerializer::new());
    Ok(())
}
```

## Integração com CQRS

`gerax-es` **não** depende de `gerax-cqrs`. A integração ocorre na direção
`gerax-cqrs → gerax-es`, através da feature opcional `event-sourcing` do
`gerax-cqrs`.

Exemplo executável:
`cargo run -p gerax-cqrs --example command_handler_es --features event-sourcing`

## Testes

```sh
cargo test -p gerax-es --all-features
cargo clippy -p gerax-es --all-features --all-targets -- -D warnings
cargo fmt --package gerax-es --check
```

## Expondo novos adapters

Implemente as portas e use os tipos concretos por composição (ou
`Arc<dyn ...>`), mantendo o restante do fluxo inalterado:

- `EventStore` — `load` / `append` (com validação de `expected_version` e de
  sequência).
- `SnapshotStore` — `load` / `save` (opcional).
- `EventPublisher` — `publish` (após persistência confirmada).
- `EventSerializer` — `serialize` / `deserialize` (formatos além de JSON).