//! # gerax-kurrentdb
//!
//! Adapter [KurrentDB](https://kurrent.io/) (antigo EventStoreDB) para
//! o ecossistema Gerax, com duas funcionalidades **independentes**:
//!
//! ```text
//! gerax-kurrentdb
//! ├── event_store     implementa gerax_es::EventStore
//! └── subscription    Persistent Subscriptions (ACK/NACK/retry/park)
//! ```
//!
//! As duas compartilham o mesmo [`KurrentDbClient`], mas criar uma não
//! exige a outra.
//!
//! ## Event Store
//!
//! - streams nomeadas como `{aggregate_type}-{aggregate_id}`;
//! - leitura em ordem crescente de revisão;
//! - append com controle de concorrência otimista;
//! - `Version 0` ↔ `StreamState::NoStream`;
//! - `Version n` ↔ `StreamRevision(n - 1)`;
//! - `event_id`, `aggregate_type`, `aggregate_id`, `occurred_at` e a
//!   metadata extensível preservados no custom metadata.
//!
//! ```rust,no_run
//! use gerax_kurrentdb::{KurrentDbClient, KurrentDbEventStore};
//! use gerax_es::EventStore;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = KurrentDbClient::connect(
//!     "kurrentdb://admin:changeit@localhost:2113?tls=false",
//! ).await?;
//! let store = KurrentDbEventStore::new(client);
//! let events = store.load("Aluno", "aluno-1").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Persistent Subscriptions
//!
//! ```rust,no_run
//! use gerax_kurrentdb::{
//!     KurrentDbClient, KurrentDbSubscription, SubscriptionAck,
//!     SubscriptionConfig, SubscriptionManager,
//! };
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = KurrentDbClient::connect(
//!     "kurrentdb://admin:changeit@localhost:2113?tls=false",
//! ).await?;
//! let config = SubscriptionConfig::all("gerax-event-relay");
//!
//! // Operação administrativa explícita, idempotente.
//! let manager = SubscriptionManager::new(client.clone());
//! manager.ensure(&config).await?;
//!
//! // Consumo independente do Event Store.
//! let subscription = KurrentDbSubscription::new(client, config)?;
//! let mut session = subscription.subscribe().await?;
//! let event = session.next_event().await?;
//! session.ack(&event, SubscriptionAck::Ack).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Event Relay e NATS
//!
//! O Event Relay **não** pertence a esta crate. Use
//! `gerax-eda::event_relay` para orquestrar o consumo e
//! `gerax-nats` para publicar no JetStream. Com a feature `eda`
//! (padrão), [`KurrentDbEventSource`] implementa a porta genérica
//! `gerax_eda::event_relay::EventSource`:
//!
//! ```text
//! KurrentDB → KurrentDbEventSource → EventRelay → NatsIntegrationPublisher
//!                                              → JetStream PubAck
//!                                              → ACK no KurrentDB
//! ```
//!
//! > O ACK no KurrentDB só ocorre depois da confirmação de publicação
//! > no NATS JetStream. A entrega é **at-least-once**; não há promessa
//! > de exactly-once entre KurrentDB e NATS.
//!
//! ## Configuração
//!
//! ```toml
//! [kurrentdb]
//! connection_string = "kurrentdb://admin:changeit@localhost:2113?tls=false"
//!
//! [kurrentdb.subscription]
//! stream = "$all"
//! group = "gerax-event-relay"
//! buffer_size = 100
//! max_retry_count = 5
//! replay_parked_interval_secs = 30
//! replay_parked_max_backoff_secs = 3600
//! ```
//!
//! Use [`KurrentDbConfig::from_toml_str`] / [`KurrentDbConfig::from_toml_file`]
//! para carregar a configuração. Credenciais devem vir de variáveis de
//! ambiente ou de um gerenciador de segredos.
//!
//! ## Replay automático de mensagens parkeadas
//!
//! [`SubscriptionManager::start_replay_parked`] inicia uma tarefa
//! periódica que reencaminha mensagens parkeadas com backoff
//! exponencial após falhas consecutivas. O handle
//! [`ReplayParkedHandle`] cancela a tarefa quando descartado.

pub mod client;
pub mod config;
pub mod error;
pub mod event_store;
pub mod subscription;

pub use client::KurrentDbClient;
pub use config::{DEFAULT_CONNECTION_STRING, KurrentDbConfig, RunProjections};
pub use error::KurrentDbError;
pub use event_store::{KurrentDbEventStore, parse_stream_name, stream_name};
pub use subscription::{
    ALL_STREAM, ConsumerStrategy, GroupState, KurrentDbSubscription, ReplayParkedHandle,
    SubscriptionAck, SubscriptionConfig, SubscriptionError, SubscriptionEvent,
    SubscriptionFilterConfig, SubscriptionInfo, SubscriptionManager, SubscriptionSession,
    SubscriptionStart,
};

#[cfg(feature = "eda")]
pub use subscription::KurrentDbEventSource;

#[cfg(feature = "eda")]
pub use gerax_eda;
