//! Persistent Subscriptions do KurrentDB.
//!
//! O módulo é independente do
//! [`KurrentDbEventStore`](crate::event_store::KurrentDbEventStore):
//! ambos podem compartilhar o mesmo [`KurrentDbClient`].
//!
//! ```text
//! KurrentDB
//!    │
//!    ▼
//! KurrentDbSubscription  (este módulo)
//!    │  SubscriptionEvent + ACK/NACK
//!    ▼
//! gerax-eda (Event Relay)
//!    │
//!    ▼
//! gerax-nats (JetStream)
//! ```
//!
//! ## Responsabilidades
//!
//! - [`SubscriptionManager`] — operações administrativas explícitas
//!   sobre grupos (criar, atualizar, listar, remover).
//! - [`KurrentDbSubscription`] — consumidor de um grupo existente.
//! - [`SubscriptionSession`] — sessão ativa com `next_event` e `ack`.
//! - [`SubscriptionConfig`] — configuração do grupo.
//! - [`SubscriptionEvent`] — evento independente do cliente.
//! - [`SubscriptionAck`] — ACK, retry, park e skip.
//! - [`ReplayParkedHandle`] — replay automático de mensagens parkeadas
//!   com backoff exponencial.

pub mod ack;
pub mod config;
pub mod consumer;
pub mod error;
pub mod event;
pub mod idempotency;
pub mod manager;
pub mod replay;

#[cfg(feature = "eda")]
pub mod relay;

pub use ack::SubscriptionAck;
pub use config::{
    ALL_STREAM, ConsumerStrategy, DEFAULT_BUFFER_SIZE, DEFAULT_MAX_RETRY_COUNT,
    DEFAULT_MESSAGE_TIMEOUT_SECS, DEFAULT_RECONNECT_DELAY_MS, SubscriptionConfig,
    SubscriptionFilterConfig, SubscriptionStart,
};
pub use consumer::{KurrentDbSubscription, SubscriptionSession};
pub use error::SubscriptionError;
pub use event::SubscriptionEvent;
pub use idempotency::{
    in_memory_idempotency_store, shared_idempotency_store, InMemoryIdempotencyStore,
    IdempotencyStore, SharedIdempotencyStore,
};
pub use manager::{GroupState, SubscriptionInfo, SubscriptionManager};
pub use replay::ReplayParkedHandle;

#[cfg(feature = "eda")]
pub use relay::{KurrentDbEventSource, subscription_ack, subscription_event_to_integration};
