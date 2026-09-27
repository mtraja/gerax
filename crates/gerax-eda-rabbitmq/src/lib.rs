//! # gerax-eda-rabbitmq
//!
//! Adapter RabbitMQ (AMQP 0-9-1) para o [`gerax-eda`]: publica e
//! consome [`EventEnvelope`]s através do [lapin], o client AMQP do
//! ecossistema Rust.
//!
//! > Este adapter vive em uma crate separada de propósito: o núcleo
//! > `gerax-eda` **não** depende de broker algum (ver
//! > [`gerax_eda::adapters`]).
//!
//! ## Mapeamento envelope → AMQP
//!
//! A metadata do [`EventEnvelope`] é traduzida para as propriedades
//! básicas da mensagem:
//!
//! ```text
//! AMQP property              EventMetadata
//! ─────────────────────────  ─────────────────────────────
//! message_id                 EventEnvelope.id (UUID)
//! type                       metadata.event_type
//! timestamp                  metadata.occurred_at (UNIX)
//! correlation_id             metadata.correlation_id
//! headers["x-causation-id"]  metadata.causation_id
//! headers["x-<key>"]         metadata.headers[<key>]
//! ```
//!
//! O corpo é o payload serializado pelo [`codec`] (JSON por padrão);
//! a chave de roteamento é sempre `metadata.event_type`.
//! Headers do usuário com prefixo `x-` são reservados pelo adapter.
//!
//! ## Semântica de entrega
//!
//! **At-least-once**: o consumidor reconhece (`ack`) a mensagem apenas
//! após o handler retornar `Ok` — falhas reenfileiram ou descartam a
//! mensagem, nunca a "perdem" silenciosamente:
//!
//! | Resultado do handler            | Ação na mensagem                     |
//! |---------------------------------|--------------------------------------|
//! | `Ok`                            | `ack`                                |
//! | `Retryable` (< max_redeliveries)| `nack(requeue = true)`               |
//! | `Retryable` (esgotado)          | `reject(requeue = false)`            |
//! | `NonRetryable`/`ProcessingFailed`| `reject(requeue = false)`           |
//!
//! Consumidores **devem ser idempotentes** por `EventEnvelope.id`
//! (a identificação existe justamente para isso).
//!
//! ## Por que não implementa `gerax_eda::EventBus`
//!
//! Publicar exige `E: Serialize` e desserializar exige
//! `E: DeserializeOwned`, bounds que a porta genérica
//! [`EventBus::publish`](gerax_eda::EventBus::publish) não pode
//! expressar (o código do evento passa pelo registry). O adapter
//! entrega a mesma semântica através de métodos inerentes e um codec
//! plugável ([`EventCodec`]).
//!
//! ## Exemplo
//!
//! ```rust,no_run
//! use std::sync::Arc;
//!
//! use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
//! use gerax_eda_rabbitmq::{
//!     JsonEventCodec, RabbitConsumerConfig, RabbitHandlerRegistry,
//!     RabbitMqConfig, RabbitMqConsumer, RabbitMqEventBus,
//! };
//!
//! #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
//! struct RegistrarDiario;
//!
//! #[async_trait::async_trait]
//! impl EventHandler<AlunoMatriculado> for RegistrarDiario {
//!     async fn handle(
//!         &self,
//!         event: EventEnvelope<AlunoMatriculado>,
//!     ) -> Result<(), EventHandlerError> {
//!         println!("{} matriculado", event.event.aluno_id);
//!         Ok(())
//!     }
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let bus = RabbitMqEventBus::connect(RabbitMqConfig::default(), JsonEventCodec).await?;
//!
//! let registry = Arc::new(RabbitHandlerRegistry::new());
//! registry.register::<AlunoMatriculado, _, _>(
//!     "aluno.matriculado",
//!     RegistrarDiario,
//!     JsonEventCodec,
//! );
//!
//! let consumer = RabbitMqConsumer::connect(
//!     RabbitMqConfig::default(),
//!     RabbitConsumerConfig::default(),
//!     registry,
//! ).await?;
//! let consumer_task = tokio::spawn(async move { consumer.consume().await });
//!
//! bus.publish_new(AlunoMatriculado {
//!     aluno_id: "a-1".into(),
//! }).await?;
//!
//! tokio::time::sleep(std::time::Duration::from_millis(500)).await;
//! consumer_task.abort();
//! # Ok(())
//! # }
//! ```
//!
//! Exemplo executável: `cargo run -p gerax-eda-rabbitmq --example aluno_matriculado`.

pub mod bus;
pub mod codec;
pub mod config;
pub mod consumer;
pub mod error;
mod map;

pub use bus::RabbitMqEventBus;
pub use codec::{EventCodec, JsonEventCodec};
pub use config::{RabbitConsumerConfig, RabbitMqConfig, UnknownEventPolicy};
pub use consumer::{RabbitHandlerRegistry, RabbitMqConsumer, RabbitMqHandlerAdapter};
pub use error::RabbitMqError;
