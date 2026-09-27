//! # `gerax-rabbitmq-stream` — Event Bus sobre RabbitMQ Streams
//!
//! Adapter que leva eventos do [`gerax_eda`] para o **RabbitMQ
//! Streams**, usando o crate oficial
//! [`rabbitmq_stream_client`](https://docs.rs/rabbitmq-stream-client).
//!
//! ```text
//! ┌──────────────────────┐
//! │       gerax-eda      │  Event / EventEnvelope / EventHandler
//! └──────────┬───────────┘
//!            │  (nenhum tipo de streams atravessa a fronteira)
//! ┌──────────▼───────────┐
//! │ gerax-rabbitmq-stream│  envelope + serialização + roteamento
//! └──────────┬───────────┘
//!            │  Message (body + application_properties)
//! ┌──────────▼───────────┐
//! │   RabbitMQ Streams   │  log append-only, port 5552
//! └──────────────────────┘
//! ```
//!
//! ## O que este adapter é
//!
//! - **Producer compartilhado** (uma conexão para N publicações) com
//!   três modos: `Async` (buffer), `Confirmed` (aguarda o broker) e
//!   `Batch`;
//! - **Consumer com offset tracking** — client-side (store próprio)
//!   ou server-side (o broker guarda o offset);
//! - **Semântica at-least-once** documentada: o offset é gravado
//!   **depois** do handler, então handlers precisam ser idempotentes;
//! - **Envelope em duas camadas** — payload no `body`, rastreabilidade
//!   em `application_properties` (que o broker indexa, habilitando
//!   filtering server-side);
//! - **Shutdown explícito** — subscriptions param antes do producer.
//!
//! ## O que este adapter **não** é
//!
//! | Não oferece | Por quê |
//! |---|---|
//! | `impl gerax_eda::EventBus` | a porta exige `E: Event + Clone`; serializar exige `E: Serialize` (ver [`RabbitMqStreamEventBus::publish`]) |
//! | *exactly-once* | não é prometido pelo RabbitMQ Streams; nem simulado |
//! | *consumer group* no estilo Kafka | cada consumidor lê independentemente e controla o próprio offset |
//! | DLQ | streams não têm rejeição seletiva; payload malformado é pulado com log explícito |
//! | Auto-criação de `TlsConfiguration` | TLS é opt-in e configurado pelo chamador ([`TlsConfig`]) |
//!
//! ## Exemplo mínimo
//!
//! ```rust,no_run
//! use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
//! use gerax_rabbitmq_stream::{
//!     ConsumerConfig, JsonEventSerializer, PublishMode, RabbitMqStreamConfig,
//!     RabbitMqStreamEventBusBuilder, StartOffset, StreamConfig,
//! };
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct AlunoMatriculado {
//!     aluno_id: String,
//! }
//!
//! impl Event for AlunoMatriculado {
//!     fn event_type(&self) -> &'static str {
//!         "aluno.matriculado"
//!     }
//! }
//!
//! # #[async_trait::async_trait]
//! # impl EventHandler<AlunoMatriculado> for Registrar {
//! #     async fn handle(&self, _e: EventEnvelope<AlunoMatriculado>) -> Result<(), EventHandlerError> { Ok(()) }
//! # }
//! struct Registrar;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let config = RabbitMqStreamConfig::builder()
//!     .host("localhost")
//!     .port(5552)                       // streams, não 5672 (AMQP)
//!     .build()?;
//!
//! // Publicação
//! let mut bus = RabbitMqStreamEventBusBuilder::new(config.clone(), StreamConfig::new("school-events"))
//!     .publish_mode(PublishMode::Confirmed)
//!     .build()
//!     .await?;
//!
//! bus.publish(EventEnvelope::new(AlunoMatriculado { aluno_id: "a-1".into() }))
//!     .await?;
//!
//! // Consumo
//! let consumer = bus
//!     .connection()
//!     .consumer(
//!         ConsumerConfig::builder()
//!             .stream("school-events")
//!             .consumer_name("school-consumer")
//!             .offset(StartOffset::First)
//!             .build()?,
//!         JsonEventSerializer,
//!     )
//!     .await?;
//!
//! consumer.register::<AlunoMatriculado, _>("aluno.matriculado", Registrar);
//!
//! let subscription = consumer.subscribe().await?;
//! bus.add_subscription(subscription);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

mod builder;
mod config;
mod connection;
mod consumer;
mod error;
mod event_bus;
mod message;
mod offset;
mod producer;
mod subscription;

pub use builder::RabbitMqStreamEventBusBuilder;
pub use config::{
    ConsumerConfig, ConsumerConfigBuilder, DEFAULT_BATCH_SIZE, DEFAULT_HOST, DEFAULT_PORT,
    DEFAULT_STREAM, PublishMode, RabbitMqStreamConfig, RabbitMqStreamConfigBuilder,
    ReconnectPolicy, StreamConfig, TlsConfig,
};
pub use connection::{RabbitMqStreamConnection, StreamOutcome};
pub use consumer::{ErasedHandler, RabbitMqStreamConsumer, StreamHandlerRegistry};
pub use error::RabbitMqStreamError;
pub use event_bus::RabbitMqStreamEventBus;
pub use message::{
    ENVELOPE_SCHEMA_VERSION, EventSerializer, JsonEventSerializer, MessageEnvelope,
    PROP_AGGREGATE_ID, PROP_AGGREGATE_TYPE, PROP_CAUSATION_ID, PROP_CONTENT_TYPE,
    PROP_CORRELATION_ID, PROP_EVENT_ID, PROP_EVENT_TYPE, PROP_HEADER_PREFIX, PROP_OCCURRED_AT,
    PROP_SCHEMA_VERSION, PROP_VERSION,
};
pub use offset::{InMemoryOffsetStore, OffsetStore, OffsetStrategy, StartOffset};
pub use producer::{PublishConfirmation, RabbitMqStreamProducer};
pub use subscription::RabbitMqStreamSubscription;

/// Reexporta o `gerax-eda` para quem só depende deste adapter.
///
/// Evita que a aplicação precise declarar as duas crates para
/// implementar um [`gerax_eda::EventHandler`].
pub use gerax_eda;

/// Reexporta o cliente oficial de streams para quem precisa da API
/// de baixo nível (filtros do broker, super streams, métricas).
///
/// A API do adapter cobre o caso comum; este reexport existe para
/// não obrigar o consumidor a declarar a mesma versão da lib.
pub use rabbitmq_stream_client;
