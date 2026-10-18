//! # gerax-nats
//!
//! Adapter de infraestrutura **NATS** (Core + JetStream) para o
//! ecossistema Gerax, usando o [async-nats] como cliente oficial.
//!
//! Implementa o contrato definido em [`gerax_eda`] (porta
//! [`EventBus`](gerax_eda::EventBus)) usando NATS como transporte:
//!
//! ```text
//! gerax-nats
//!      │
//!      ▼
//!  gerax-eda          (o contrário nunca acontece)
//! ```
//!
//! ```text
//!                Gerax Application
//!                       │
//!                       ▼
//!                  EventBus                    (gerax-eda)
//!                       │
//!                       ▼
//!                  NatsEventBus                (gerax-nats)
//!                       │
//!             ┌─────────┴─────────┐
//!             │                   │
//!        Core NATS           JetStream
//!             │                   │
//!      request/reply          Stream
//!      notifications          Consumer durável
//!      transitório            ACK / redelivery / replay / DLQ
//!                                 │
//!                                 ▼
//!                             NATS Server
//! ```
//!
//! Trocar RabbitMQ por NATS (ou vice-versa) é uma decisão de
//! infraestrutura, não uma mudança no domínio: ambos os adapters
//! implementam a mesma porta [`EventBus`](gerax_eda::EventBus).
//!
//! # Exemplo
//!
//! ```rust,no_run
//! use gerax_eda::{Event, EventEnvelope, EventBus};
//! use gerax_nats::{NatsConfig, NatsEventBus};
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
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let config = NatsConfig::builder()
//!     .url("nats://localhost:4222")
//!     .build()?;
//!
//! let event_bus = NatsEventBus::connect(config).await?;
//!
//! // Pela porta EventBus — sem saber que NATS está por baixo.
//! event_bus
//!     .publish(EventEnvelope::new(AlunoMatriculado {
//!         aluno_id: "a-1".into(),
//!     }))
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Subjects (`§9`)
//!
//! Formato: `gerax.<bounded_context>.<aggregate>.<event>`. O adapter
//! compõe `"<prefixo>.<event_type>"` — o prefixo vem de
//! [`NatsConfig::subject_prefix`] e o `event_type` é o nome lógico do
//! domínio (ex.: `"clinic.patient.created"`):
//!
//! ```text
//! gerax.clinic.patient.created
//! gerax.clinic.appointment.cancelled
//! gerax.school.aluno.matriculado
//! ```
//!
//! Nenhuma string de subject é espalhada pelo código: tudo passa por
//! [`SubjectBuilder`], com validação de tokens NATS.
//!
//! # Retry e redelivery (`§13`)
//!
//! O retry **não** é implementado em loop manual: falha do handler →
//! no-ACK → o próprio JetStream reentrega após `ack_wait`, até
//! `max_deliver` ([`NatsConsumerConfig`]). Core NATS é at-most-once
//! por natureza (sem redelivery).
//!
//! # Dead Letter (`§14`)
//!
//! Payload indecifrável ou falhas além de `max_deliver` são
//! encaminhados a `<prefixo>.dlq.<service>.<event>` ([`DlqMessage`]
//! preserva payload original, `event_id`, subject, tentativa, erro,
//! timestamp e correlação). Habilitar via
//! [`NatsConsumerConfigBuilder::dead_letter`].
//!
//! # Idempotência (`§15`)
//!
//! Entrega é **at-least-once**: consumidores devem deduplicar por
//! `event_id`. O adapter publica `Nats-Msg-Id` (= `event_id`), o que
//! ativa a deduplicação nativa do JetStream (`duplicate_window`) no
//! produtor; no consumidor, a deduplicação é responsabilidade da
//! aplicação (ex.: inbox no banco — a trait fica fora deste crate).
//!
//! # Request/reply (`§17`)
//!
//! Disponível em [`NatsClient::request`] — síncrono, com timeout, para
//! consultas (`patient.get`); **não** substitui eventos assíncronos.
//!
//! # Orbit.rs (`§18`, Fase 6) e performance (`§27`, Fase 7)
//!
//! A primeira versão **não** adota Orbit.rs nem otimizações de
//! throughput (batch/atomic publish, fast ingest, partitioned consumer
//! groups): nenhuma extensão foi necessária para os critérios de
//! aceite (`§28`), e o `async-nats` resolve toda a fundação. Quando um
//! requisito real aparecer (request-many, atomic publish, contadores
//! distribuídos), a extensão Orbit entra de forma incremental atrás de
//! uma API do Gerax — sem esconder o `async-nats`.
//!
//! # Compatibilidade (`§25`)
//!
//! Testado contra `async-nats` 0.50 (API de consumers `pull::*`).
//! Funcionalidades JetStream recentes podem exigir versões mínimas do
//! `nats-server` (consumidores duráveis: >= 2.3; `filter_subjects`
//! múltiplo: >= 2.9 — janela `duplicate_window` é nativa).
//!
//! # Feature flags (`§26`)
//!
//! ```toml
//! default = ["jetstream"]
//! jetstream = ["async-nats/jetstream"]
//! ```
//!
//! Com `--no-default-features`, o crate oferece apenas Core NATS:
//! [`NatsClient`] (publish/subscribe/request) e [`NatsEventBus`] em
//! modo transitório ([`NatsConsumer`] e [`NatsJetStream`] exigem a
//! feature `jetstream`).
//!
//! [async-nats]: https://crates.io/crates/async-nats

pub mod client;
pub mod config;
pub mod envelope;
pub mod error;
pub mod event_bus;
pub mod publisher;
pub mod subject;

#[cfg(feature = "jetstream")]
pub mod consumer;
#[cfg(feature = "jetstream")]
pub mod jetstream;

pub use client::NatsClient;
pub use config::{
    DEFAULT_NATS_URL, DEFAULT_STREAM_NAME, JetStreamConfig, NatsConfig, StreamStorage,
};
pub use envelope::NatsEnvelope;
pub use error::NatsError;
pub use event_bus::NatsEventBus;
pub use publisher::{NatsPublisher, PublishMode};
pub use subject::SubjectBuilder;

#[cfg(feature = "jetstream")]
pub use consumer::{
    DlqConfig, DlqMessage, NatsConsumer, NatsConsumerConfig, NatsConsumerConfigBuilder,
    NatsDelivered,
};
#[cfg(feature = "jetstream")]
pub use jetstream::NatsJetStream;
