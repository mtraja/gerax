# Skill: Implementar `gerax-es`

## Objetivo

Implementar a crate `gerax-es` (Gerax Event Sourcing), fornecendo uma infraestrutura genérica, type-safe e extensível de Event Sourcing em Rust para ser utilizada opcionalmente pelo `gerax-cqrs`.

A implementação deve separar claramente:

- Domain
- Aggregate
- Domain Events
- Event Store
- Aggregate Repository
- Event serialization
- Optimistic concurrency
- Snapshots
- Metadata
- Projections
- Event publishing

O `gerax-es` **não deve depender de `gerax-cqrs`**.

A dependência deve ser:

```text
gerax-cqrs
      │
      ▼
    gerax-es
      │
      ├── Event Store
      ├── Aggregate Repository
      ├── Aggregate Root
      ├── Domain Events
      └── Snapshots
```

O objetivo é permitir:

```text
Command
   │
   ▼
CommandHandler
   │
   ▼
Aggregate Repository
   │
   ├── load aggregate
   │       │
   │       ▼
   │   Event Store
   │       │
   │       ▼
   │   Events
   │
   ▼
Aggregate
   │
   │ execute command
   ▼
New Events
   │
   ▼
Event Store
   │
   ▼
Event Bus / Projections
```

---

# 1. Regras fundamentais

O agente DEVE seguir estas regras.

### 1.1 `gerax-es` é independente de CQRS

Nunca adicionar:

```rust
gerax-cqrs
```

como dependência de:

```text
gerax-es
```

A relação correta é:

```text
gerax-cqrs → gerax-es
```

e não:

```text
gerax-es → gerax-cqrs
```

### 1.2 O domínio não conhece infraestrutura

O Aggregate não deve conhecer:

- PostgreSQL
- MongoDB
- Kafka
- NATS
- Redis
- HTTP
- Axum
- Actix
- Tokio

O domínio deve trabalhar somente com:

```rust
Aggregate
Event
Command/domain operation
Version
```

### 1.3 Event Store é uma porta

O Event Store deve ser definido através de trait.

Implementações concretas devem ficar em crates/adapters separados.

Exemplo futuro:

```text
gerax-es
gerax-es-memory
gerax-es-postgres
gerax-es-mongodb
```

### 1.4 Event Sourcing é a fonte de verdade

O estado persistido do Aggregate não é a fonte primária.

A fonte primária é:

```text
Event Stream
```

O Aggregate pode ser reconstruído através de:

```text
initial state
    +
event 1
    +
event 2
    +
event 3
    ...
    =
current state
```

---

# 2. Estrutura da crate

Criar inicialmente:

```text
gerax-es/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── aggregate.rs
    ├── event.rs
    ├── event_store.rs
    ├── repository.rs
    ├── snapshot.rs
    ├── error.rs
    ├── metadata.rs
    └── version.rs
```

Estrutura futura:

```text
gerax-es/
├── src/
│   ├── aggregate/
│   ├── event/
│   ├── store/
│   ├── repository/
│   ├── snapshot/
│   ├── projection/
│   └── bus/
```

Não implementar infraestrutura específica de banco dentro de `gerax-es`.

---

# 3. Aggregate Root

Criar uma abstração para Aggregate Root.

O Aggregate deve possuir:

```rust
id
version
state
pending events
```

A API conceitual deve permitir:

```rust
let mut aggregate = Student::empty(id);

aggregate.execute(command)?;

let events = aggregate.take_events();
```

O Aggregate deve conseguir:

1. ser criado vazio;
2. aplicar eventos históricos;
3. gerar novos eventos;
4. manter eventos pendentes;
5. controlar sua versão.

---

# 4. Aggregate trait

Criar um trait semelhante a:

```rust
pub trait Aggregate: Sized {
    type Id;
    type Event;
    type Error;

    fn aggregate_type() -> &'static str;

    fn empty(id: Self::Id) -> Self;

    fn id(&self) -> &Self::Id;

    fn version(&self) -> Version;

    fn apply(&mut self, event: &Self::Event) -> Result<(), Self::Error>;

    fn raise(&mut self, event: Self::Event) -> Result<(), Self::Error>;

    fn take_events(&mut self) -> Vec<Self::Event>;
}
```

A assinatura pode ser ajustada se necessário para obter uma API mais idiomática.

Não introduzir complexidade desnecessária.

---

# 5. Separar `apply` de `raise`

Esta distinção é obrigatória.

## `apply`

Usado para eventos históricos:

```rust
aggregate.apply(&event)?;
```

Não deve colocar o evento novamente na lista de eventos pendentes.

Exemplo:

```text
Event Store
    │
    ▼
Event 1
    │
    ▼
apply(Event 1)

Event 2
    │
    ▼
apply(Event 2)
```

Nenhum evento deve ser produzido novamente.

## `raise`

Usado quando o Aggregate produz um novo evento:

```rust
aggregate.raise(StudentCreated { ... })?;
```

Internamente:

```text
raise(event)
    │
    ├── apply(event)
    │
    └── pending_events.push(event)
```

Assim:

```text
historical events
      │
      ▼
    apply
      │
      ▼
 current state


new command
      │
      ▼
    raise
      │
      ├── apply
      └── pending_events
```

---

# 6. Version

Criar um tipo dedicado:

```rust
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
pub struct Version(u64);
```

Fornecer:

```rust
Version::initial()
Version::new(value)
value()
next()
```

A versão representa a posição do Aggregate no Event Stream.

Exemplo:

```text
version 0
    │
    ├── StudentCreated
    │
version 1
    │
    ├── StudentRenamed
    │
version 2
```

---

# 7. Domain Event

O `gerax-es` deve fornecer abstrações para eventos.

Criar uma separação entre:

```text
Domain Event
```

e:

```text
Stored Event
```

Um Domain Event representa o evento do domínio.

Um Stored Event representa o evento persistido.

Exemplo:

```rust
StudentCreated
StudentRenamed
StudentEnrolled
```

não devem conhecer:

```text
event_id
occurred_at
stream_position
database id
```

Esses dados pertencem ao armazenamento.

---

# 8. StoredEvent

Criar uma estrutura semelhante a:

```rust
pub struct StoredEvent {
    pub event_id: EventId,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub event_type: String,
    pub version: Version,
    pub payload: serde_json::Value,
    pub metadata: EventMetadata,
    pub occurred_at: DateTime<Utc>,
}
```

Os tipos podem ser abstraídos posteriormente.

O importante é preservar:

- identidade do evento;
- identidade do Aggregate;
- tipo do Aggregate;
- tipo do evento;
- versão;
- payload;
- metadata;
- timestamp.

---

# 9. EventId

Criar um identificador próprio para eventos.

Preferencialmente:

```rust
Uuid
```

através de um newtype:

```rust
pub struct EventId(Uuid);
```

Não espalhar `Uuid` diretamente por toda a API pública quando um tipo semântico for mais apropriado.

---

# 10. EventMetadata

Criar:

```rust
pub struct EventMetadata {
    ...
}
```

A metadata deve permitir posteriormente informações como:

```text
correlation_id
causation_id
user_id
tenant_id
request_id
```

Não adicionar campos desnecessários sem necessidade.

Uma implementação inicial pode utilizar um mapa:

```rust
HashMap<String, serde_json::Value>
```

ou uma estrutura extensível equivalente.

---

# 11. EventStore

Criar o port principal:

```rust
#[async_trait]
pub trait EventStore {
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, EventStoreError>;

    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: Version,
        events: Vec<StoredEvent>,
    ) -> Result<(), EventStoreError>;
}
```

A API pode ser refinada para utilizar tipos fortemente tipados.

O Event Store deve suportar:

```text
load stream
append events
optimistic concurrency
```

---

# 12. Optimistic Concurrency

Esta é uma funcionalidade obrigatória.

Quando o Aggregate é carregado:

```text
version = 5
```

O CommandHandler modifica o Aggregate e tenta salvar.

Deve informar:

```text
expected_version = 5
```

Se outro processo já persistiu um evento:

```text
actual_version = 6
```

o Event Store deve rejeitar o append.

Erro:

```rust
ConcurrencyError {
    expected: Version,
    actual: Version,
}
```

Fluxo:

```text
Process A                 Process B

load version 5            load version 5

modify                     modify

append expected 5
      │
      ▼
version 6

                          append expected 5
                                │
                                ▼
                         CONCURRENCY ERROR
```

Nunca sobrescrever silenciosamente eventos concorrentes.

---

# 13. AggregateRepository

Criar uma abstração:

```rust
#[async_trait]
pub trait AggregateRepository<A>
where
    A: Aggregate,
{
    async fn load(
        &self,
        id: &A::Id,
    ) -> Result<A, RepositoryError>;

    async fn save(
        &self,
        aggregate: &mut A,
    ) -> Result<(), RepositoryError>;
}
```

O repository deve encapsular:

```text
EventStore
   │
   ▼
load events
   │
   ▼
rehydrate aggregate
```

e:

```text
Aggregate
   │
   ▼
pending events
   │
   ▼
EventStore.append()
```

---

# 14. Rehydration

O Repository deve reconstruir o Aggregate assim:

```text
Event Store
    │
    ▼
StoredEvent[]
    │
    ▼
deserialize
    │
    ▼
DomainEvent
    │
    ▼
Aggregate::apply()
    │
    ▼
Aggregate atual
```

A versão deve ser restaurada de acordo com o último evento aplicado.

---

# 15. Serialização de eventos

O Event Store não deve depender diretamente de um enum específico.

Criar uma estratégia de serialização/deserialização.

O mínimo necessário:

```rust
trait EventSerializer {
    type Event;
    type Error;

    fn serialize(
        &self,
        event: &Self::Event,
    ) -> Result<serde_json::Value, Self::Error>;

    fn deserialize(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<Self::Event, Self::Error>;
}
```

A solução pode posteriormente suportar:

```text
JSON
MessagePack
Bincode
Wincode
```

Não acoplar `gerax-es` a um único formato de persistência.

---

# 16. Event Registry

Criar posteriormente uma abstração para registrar tipos de eventos.

Exemplo conceitual:

```rust
registry.register::<StudentCreated>();
registry.register::<StudentRenamed>();
registry.register::<StudentEnrolled>();
```

O Registry deve permitir:

```text
event_type → deserializer
```

Esse mecanismo deverá facilitar futuramente macros como:

```rust
#[derive(Event)]
struct StudentCreated {
    ...
}
```

Não implementar macros obrigatoriamente na primeira versão, mas projetar a API para permitir isso.

---

# 17. Integração com `gerax-cqrs`

O `gerax-cqrs` deve poder fazer:

```text
Command
   │
   ▼
CommandHandler
   │
   ▼
Repository.load()
   │
   ▼
Aggregate
   │
   ▼
command
   │
   ▼
raise events
   │
   ▼
Repository.save()
   │
   ▼
EventStore.append()
```

Exemplo conceitual:

```rust
pub struct CreateStudentHandler<R> {
    repository: R,
}
```

O handler:

```rust
async fn handle(
    &self,
    command: CreateStudent,
) -> Result<StudentId, Error> {
    let mut student = Student::empty(command.student_id);

    student.create(
        command.name,
    )?;

    self.repository.save(&mut student).await?;

    Ok(student.id().clone())
}
```

O `gerax-cqrs` deve conhecer apenas a interface do Repository.

---

# 18. Não colocar Command dentro do `gerax-es`

Não criar:

```rust
Command
CommandHandler
CommandBus
Query
QueryHandler
QueryBus
```

dentro de `gerax-es`.

Esses conceitos pertencem ao:

```text
gerax-cqrs
```

O Event Sourcing deve permanecer focado em:

```text
Aggregate
Event
EventStore
Repository
Snapshot
Projection infrastructure
```

---

# 19. Snapshot

Projetar suporte a Snapshot.

Um Snapshot representa:

```text
Aggregate state at version N
```

Exemplo:

```text
Event 1
Event 2
...
Event 5000
```

Sem snapshot:

```text
load 5000 events
```

Com snapshot:

```text
Snapshot version 4500
       +
Events 4501..5000
```

Criar inicialmente apenas a abstração:

```rust
#[async_trait]
pub trait SnapshotStore {
    async fn load(...);

    async fn save(...);
}
```

Não tornar Snapshot obrigatório.

---

# 20. Projections

O `gerax-es` deve ser projetado para permitir Projections.

Uma Projection consome eventos:

```text
Event Store
    │
    ▼
Event
    │
    ├── Projection A
    │
    ├── Projection B
    │
    └── Event Bus
```

Exemplos:

```text
StudentProjection
EnrollmentProjection
FinancialProjection
```

Não misturar Projection com Aggregate.

Projection não altera Aggregate.

---

# 21. Event Publisher

Projetar uma abstração para publicação após persistência:

```rust
#[async_trait]
pub trait EventPublisher {
    async fn publish(
        &self,
        events: &[StoredEvent],
    ) -> Result<(), PublishError>;
}
```

A ordem correta deve ser:

```text
Aggregate
    │
    ▼
Event Store
    │
    │ persistência confirmada
    ▼
Event Publisher
    │
    ▼
Consumers / Projections
```

Nunca publicar o evento antes de confirmar a persistência.

---

# 22. Outbox

Não implementar Outbox diretamente no núcleo de `gerax-es`.

Porém, projetar a API para permitir posteriormente:

```text
Aggregate
   │
   ▼
Transaction
   ├── Event Store
   └── Outbox
```

Isso será importante para adapters PostgreSQL/MongoDB.

---

# 23. Event Store em memória

Implementar primeiro:

```text
InMemoryEventStore
```

Ela será usada para:

- testes;
- exemplos;
- desenvolvimento;
- validação da API.

Deve suportar:

```rust
load()
append()
optimistic concurrency
```

Exemplo:

```rust
let store = InMemoryEventStore::new();
```

---

# 24. Testes

A implementação deve ser orientada por testes.

Criar testes para:

### Aggregate

- criação;
- aplicação de eventos;
- geração de eventos;
- pending events;
- versionamento.

### Repository

- aggregate inexistente;
- aggregate existente;
- rehydration;
- save;
- múltiplos eventos.

### Event Store

- append;
- load;
- stream vazio;
- versões;
- concorrência otimista;
- append fora de ordem.

### Serialization

- serialize;
- deserialize;
- evento desconhecido;
- payload inválido.

### Snapshot

- save;
- load;
- versão do snapshot.

---

# 25. Exemplo obrigatório

Criar um exemplo mínimo:

```text
Student
```

com eventos:

```rust
StudentCreated
StudentRenamed
```

Fluxo:

```text
CreateStudent
      │
      ▼
Student::empty()
      │
      ▼
raise(StudentCreated)
      │
      ▼
Repository.save()
      │
      ▼
EventStore
```

Depois:

```text
load Student
      │
      ▼
StudentCreated
      │
      ▼
Student::apply()
      │
      ▼
Student
      │
      ▼
raise(StudentRenamed)
      │
      ▼
save
```

---

# 26. API pública

`lib.rs` deve expor somente a API necessária.

Exemplo:

```rust
pub mod aggregate;
pub mod error;
pub mod event;
pub mod event_store;
pub mod metadata;
pub mod repository;
pub mod snapshot;
pub mod version;

pub use aggregate::Aggregate;
pub use event::{EventId, StoredEvent};
pub use event_store::EventStore;
pub use repository::AggregateRepository;
pub use version::Version;
```

Evitar expor detalhes internos.

---

# 27. Error handling

Criar erros específicos.

Exemplo:

```rust
pub enum EventStoreError {
    Concurrency {
        expected: Version,
        actual: Version,
    },

    Serialization(...),

    Storage(...),

    EventNotFound(...),

    Other(...),
}
```

Separar:

```text
AggregateError
EventStoreError
RepositoryError
SerializationError
SnapshotError
PublishError
```

Evitar:

```rust
Box<dyn Error>
```

como erro principal da API.

---

# 28. Async

Operações de infraestrutura devem ser async:

```rust
load()
append()
save()
publish()
```

Usar:

```rust
async_trait
```

quando necessário.

Não tornar operações puramente de domínio async sem necessidade.

O Aggregate deve continuar sendo síncrono:

```rust
aggregate.apply(...)
aggregate.raise(...)
```

---

# 29. Thread safety

As abstrações de infraestrutura devem ser projetadas para uso compartilhado:

```rust
Arc<dyn EventStore>
```

quando apropriado.

Implementações concretas devem ser compatíveis com:

```rust
Send + Sync
```

quando exigido pela aplicação.

Não introduzir `Mutex`/`RwLock` indiscriminadamente.

---

# 30. Compatibilidade com Rust

Respeitar a versão Rust definida no workspace Gerax:

```text
edition = 2024
rust-version = 1.96.1
```

Manter compatibilidade com o workspace existente.

Não alterar versões globais sem necessidade.

---

# 31. Dependências

Manter `gerax-es` pequeno.

Dependências esperadas:

```text
async-trait
serde
serde_json
uuid
chrono
thiserror
```

Adicionar outras somente quando justificadas.

Não adicionar:

```text
sqlx
mongodb
redis
kafka
nats
axum
actix
```

ao núcleo.

Essas dependências pertencem aos adapters.

---

# 32. Workspace

Integrar:

```toml
[workspace]
members = [
    ...
    "gerax-es",
]
```

Não quebrar os crates existentes.

O `gerax-cqrs` deve poder adicionar futuramente:

```toml
gerax-es = { path = "../gerax-es" }
```

sem que `gerax-es` precise conhecer CQRS.

---

# 33. Critérios de aceitação

A implementação somente será considerada concluída quando:

- [ ] `gerax-es` compila;
- [ ] testes passam;
- [ ] Aggregate pode ser reidratado;
- [ ] eventos podem ser gerados;
- [ ] eventos pendentes podem ser recuperados;
- [ ] Event Store possui `load`;
- [ ] Event Store possui `append`;
- [ ] optimistic concurrency funciona;
- [ ] Repository funciona;
- [ ] existe InMemoryEventStore;
- [ ] eventos podem ser serializados;
- [ ] eventos podem ser desserializados;
- [ ] erros são tipados;
- [ ] domínio não depende de infraestrutura;
- [ ] `gerax-es` não depende de `gerax-cqrs`;
- [ ] `gerax-cqrs` pode utilizar `gerax-es`;
- [ ] API pública está documentada;
- [ ] exemplos existem;
- [ ] testes de integração existem.

---

# 34. Processo que o agente deve seguir

Antes de modificar código:

1. Inspecionar o workspace Gerax.
2. Ler `Cargo.toml` raiz.
3. Ler `gerax-core`.
4. Ler `gerax-cqrs`, caso já exista.
5. Identificar convenções de erros.
6. Identificar convenções de traits.
7. Identificar convenções de módulos.
8. Verificar versões das dependências.
9. Não duplicar abstrações existentes.

Depois:

```text
FASE 1
estrutura da crate

FASE 2
Version/Event/EventMetadata

FASE 3
Aggregate

FASE 4
EventStore

FASE 5
InMemoryEventStore

FASE 6
Repository

FASE 7
serialization/event registry

FASE 8
snapshot abstractions

FASE 9
integration tests

FASE 10
integração com gerax-cqrs
```

Executar testes após cada fase relevante.

---

# 35. Regras de design

O agente NÃO deve:

- criar uma arquitetura excessivamente abstrata;
- introduzir macros antes da API base estar estável;
- acoplar o domínio a banco;
- acoplar `gerax-es` a `gerax-cqrs`;
- implementar CQRS dentro do ES;
- implementar infraestrutura de PostgreSQL dentro do core;
- usar `Any` indiscriminadamente;
- usar `Box<dyn Any>` como solução padrão;
- esconder erros de concorrência;
- sobrescrever eventos existentes;
- modificar eventos históricos;
- publicar eventos antes da persistência.

O agente DEVE:

- preferir tipos fortes;
- usar associated types quando apropriado;
- manter traits pequenos;
- manter o domínio síncrono;
- manter infraestrutura async;
- preservar optimistic concurrency;
- escrever testes;
- documentar decisões arquiteturais;
- manter a API extensível sem criar abstrações prematuras.

---

# 36. Resultado arquitetural esperado

Ao final, a arquitetura deve permitir:

```text
                    ┌──────────────────┐
                    │   gerax-cqrs     │
                    │                  │
                    │ CommandHandler   │
                    │ CommandBus       │
                    └────────┬─────────┘
                             │
                             ▼
                    ┌──────────────────┐
                    │     gerax-es     │
                    │                  │
                    │ Aggregate        │
                    │ Repository       │
                    │ EventStore       │
                    │ SnapshotStore    │
                    └────────┬─────────┘
                             │
                 ┌───────────┼───────────┐
                 ▼           ▼           ▼
          InMemoryStore   PostgreSQL   MongoDB
                              │
                              ▼
                           Outbox
                              │
                              ▼
                         Event Bus
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
               Projection A        Projection B
                    │                   │
                    ▼                   ▼
                Read Model          Read Model
```

O princípio central é:

```text
CQRS decide COMO a aplicação processa Commands/Queries.

Event Sourcing decide COMO o estado do Aggregate é persistido.

gerax-cqrs pode usar gerax-es,
mas gerax-es não deve conhecer gerax-cqrs.
```

---

# 37. Entrega final do agente

Ao terminar a implementação, o agente deve apresentar:

### Arquivos criados

Lista dos arquivos.

### API principal

Mostrar:

```rust
Aggregate
EventStore
AggregateRepository
Version
StoredEvent
SnapshotStore
```

### Exemplo de uso

Mostrar um Aggregate completo com:

```text
create
load
rehydrate
command
raise event
save
```

### Testes

Informar:

```text
cargo test
cargo check
cargo clippy
```

e seus resultados.

### Decisões arquiteturais

Explicar brevemente:

- como funciona o Aggregate;
- como funciona a concorrência;
- como os eventos são serializados;
- como o Repository utiliza EventStore;
- como `gerax-cqrs` integra com `gerax-es`;
- quais partes foram deixadas para adapters futuros.

### Próximos adapters

Não implementar automaticamente, mas deixar a arquitetura preparada para:

```text
gerax-es-postgres
gerax-es-mongodb
gerax-es-memory
```

e posteriormente:

```text
gerax-eventbus-kafka
gerax-eventbus-nats
```