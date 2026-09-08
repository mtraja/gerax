//! # gerax-es
//!
//! Infraestrutura genérica e extensível de Event Sourcing para o ecossistema Gerax.
//!
//! ## Visão geral
//!
//! ```text
//! Domain (Aggregate)
//!   │  raise events (síncrono, sem IO)
//!   ▼
//! AggregateRepository        crate::repository
//!   │  load / save
//!   ▼
//! EventStore                 crate::event_store —— porta (adapters)
//!   │  append (optimistic concurrency)
//!   ▼
//! EventPublisher             crate::publisher —— porta
//!   │
//!   ▼
//! Consumers / Projections
//! ```
//!
//! ## API pública
//!
//! ### Aggregate
//!
//! - [`Aggregate`] — raiz do Aggregate: estado, versão e eventos pendentes.
//!   Domínio é **síncrono**; a infraestrutura (repository, store) é **async**.
//! - [`Version`] — posição do Aggregate no Event Stream (0 = vazio).
//! - [`StoredEvent`] / [`EventId`] — evento **persistido** no Event Store.
//! - [`EventMetadata`] — metadata extensível (causation/correlation future).
//!
//! ### Portas de infraestrutura
//!
//! - [`EventStore`] — persistência de eventos com optimistic concurrency.
//!   É uma **porta**; implementações concretas (PostgreSQL, MongoDB etc.)
//!   vivem em adapters. [`InMemoryEventStore`] serve para testes/protótipos.
//! - [`AggregateRepository`] — reidrata e persiste Aggregates.
//!   [`EventSourcedRepository`] é a implementação genérica (store + serializer).
//! - [`SnapshotStore`] / [`Snapshot`] — snapshots **opcionais** para
//!   reidratação rápida; não são aplicados automaticamente pelo repository.
//! - [`EventPublisher`] — publica eventos **após** a persistência confirmada.
//!
//! ### Serialização
//!
//! - [`EventSerializer`] — porta de serialização.
//! - [`JsonEventSerializer`] — implementação JSON para eventos com tag
//!   interna `#[serde(tag = "event_type")]`.
//! - [`SerializedEvent`] — resultado intermediário (`event_type` + `payload`).
//!
//! ### Erros
//!
//! - [`EventStoreError`] / [`ConcurrencyError`] — persistência + conflitos.
//! - [`RepositoryError`] — falhas do repository (inclui erros do store).
//! - [`SerializationError`] — falhas de serialização.
//! - [`SnapshotStoreError`] — falhas do snapshot store.
//! - [`PublishError`] — falhas de publicação.
//!
//! ## Exemplo completo
//!
//! Veja `crates/gerax-es/examples/student.rs` (Student Aggregate):
//!
//! ```text
//! CreateStudent → StudentCreated → persist
//! Load Student  → rehydrate
//! Rename        → StudentRenamed → persist
//! ```
//!
//! ```bash
//! cargo run -p gerax-es --example student
//! ```
//!
//! ## Integração com CQRS
//!
//! `gerax-es` **não** depende de `gerax-cqrs`. A integração ocorre na
//! direção `gerax-cqrs → gerax-es`, através da feature opcional
//! `event-sourcing` do crate `gerax-cqrs`:
//!
//! ```text
//! gerax-cqrs
//!   │
//!   ▼
//! Command / Query Handler
//!   │
//!   ▼
//! gerax-es Repository
//!   │
//!   ▼
//! Aggregate
//!   │
//!   ▼
//! Event Store
//! ```
//!
//! Exemplo executável: `crates/gerax-cqrs/examples/command_handler_es.rs`
//! (`cargo run -p gerax-cqrs --example command_handler_es --features event-sourcing`).

pub mod aggregate;
pub mod error;
pub mod event;
pub mod event_store;
pub mod memory;
pub mod metadata;
pub mod publisher;
pub mod repository;
pub mod serialization;
pub mod snapshot;
pub mod version;

pub use aggregate::Aggregate;
pub use error::{
    ConcurrencyError, EventStoreError, PublishError, RepositoryError, SerializationError,
    SnapshotStoreError,
};
pub use event::{EventId, StoredEvent};
pub use event_store::EventStore;
pub use memory::InMemoryEventStore;
pub use metadata::EventMetadata;
pub use publisher::EventPublisher;
pub use repository::{AggregateRepository, EventSourcedRepository};
pub use serialization::{EventSerializer, JsonEventSerializer, SerializedEvent};
pub use snapshot::{Snapshot, SnapshotStore};
pub use version::Version;
