//! Event Relay: ponte entre uma origem de eventos persistidos e um
//! destino de integração.
//!
//! ```text
//! EventSource ──► EventRelay ──► EventDispatcher ──► IntegrationPublisher
//! (KurrentDB)      (retry/ACK)     (roteamento)        (NATS JetStream)
//! ```
//!
//! A EDA define apenas contratos. Os adapters concretos vivem nas
//! crates de infraestrutura:
//!
//! | Contrato | Implementação |
//! |---|---|
//! | [`EventSource`] | `gerax-kurrentdb` (`KurrentDbEventSource`) |
//! | [`IntegrationPublisher`] | `gerax-nats` (`NatsIntegrationPublisher`) |
//! | [`IntegrationEventHandler`] | aplicação (idempotente) |
//!
//! # Regra de confirmação
//!
//! ```text
//! publicar ──► confirmado? ──sim──► ACK na origem
//!                   │
//!                   não ──► Retry/Park (sem ACK)
//! ```
//!
//! O ACK na origem **nunca** precede a confirmação do destino. A
//! entrega é at-least-once: duplicatas são possíveis e consumidores
//! devem ser idempotentes.

pub mod dispatcher;
pub mod envelope;
pub mod handler;
pub mod publisher;
pub mod relay;
pub mod source;

pub use dispatcher::{EventDispatch, EventDispatcher};
pub use envelope::IntegrationEvent;
pub use handler::{HandlerError, IntegrationEventHandler};
pub use publisher::{IntegrationPublisher, PublishError, PublisherHandler};
pub use relay::{
    EventRelay, PermanentFailureAction, RelayError, RelayOutcome, RelayRetryPolicy, RelayStats,
};
pub use source::{DeliveryAction, EventSource, SourceError, SourceEvent};
