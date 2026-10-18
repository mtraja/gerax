//! Adapter de infraestrutura **RabbitMQ** para o ecossistema Gerax.
//!
//! Implementa o contrato definido em [`gerax_eda`] (EventBus /
//! EventHandler) usando RabbitMQ como transporte:
//!
//! ```text
//! gerax-rabbitmq
//!        │
//!        ▼
//!    gerax-eda          (o contrário nunca acontece — §41)
//! ```
//!
//! # Exemplo
//!
//! ```rust,no_run
//! use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
//! use gerax_rabbitmq::{
//!     RabbitConsumerConfig, RabbitHandlerRegistry, RabbitMqConfig,
//!     RabbitMqConnection, RabbitMqEventBus, RabbitMqConsumer,
//! };
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct AlunoCriado {
//!     aluno_id: String,
//! }
//!
//! impl Event for AlunoCriado {
//!     fn event_type(&self) -> &'static str {
//!         "aluno.criado"
//!     }
//! }
//!
//! struct RegistrarAluno;
//!
//! #[async_trait::async_trait]
//! impl EventHandler<AlunoCriado> for RegistrarAluno {
//!     async fn handle(
//!         &self,
//!         _event: EventEnvelope<AlunoCriado>,
//!     ) -> Result<(), EventHandlerError> {
//!         Ok(())
//!     }
//! }
//!
//! # async fn use_case() -> Result<(), Box<dyn std::error::Error>> {
//! # use std::sync::Arc;
//! let config = RabbitMqConfig::builder()
//!     .url("amqp://guest:guest@localhost:5672/%2f")
//!     .exchange("gerax.events")
//!     .build()?;
//!
//! let connection = RabbitMqConnection::connect(config.clone()).await?;
//!
//! let event_bus = RabbitMqEventBus::from_connection(&connection, config.clone()).await?;
//!
//! event_bus.publish_event(AlunoCriado { aluno_id: "a-1".into() }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Porta `gerax_eda::EventBus`
//!
//! [`RabbitMqEventBus`] implementa a porta [`gerax_eda::EventBus`]:
//! o bound `E: Event + Clone + Serialize` do `publish` da porta é o
//! mesmo que o adapter precisa para serializar o payload ao wire. O
//! método inerente [`RabbitMqEventBus::publish_event`] oferece a
//! mesma publicação com o erro tipado [`RabbitMqError`] — ver
//! [`bus`].

// Módulos (topology fica privado — internals não fazem parte da API
// pública, `§28`).
mod topology;

pub mod bus;
pub mod config;
pub mod connection;
pub mod consumer;
pub mod envelope;
pub mod error;
pub mod publisher;

pub use bus::RabbitMqEventBus;
pub use config::{
    DEFAULT_AMQP_URI, DEFAULT_DLQ, DEFAULT_DLX, DEFAULT_EXCHANGE, DEFAULT_QUEUE, DeadLetterConfig,
    DeliveryMode, ExchangeType, RabbitConsumerConfig, RabbitMqConfig, RetryPolicy,
    UnknownEventPolicy,
};
pub use connection::RabbitMqConnection;
pub use consumer::{ErasedHandler, HandlerAdapter, RabbitHandlerRegistry, RabbitMqConsumer};
pub use envelope::{EnvelopeCodec, JsonEnvelopeCodec, MessageEnvelope};
pub use error::RabbitMqError;
pub use publisher::RabbitMqPublisher;
