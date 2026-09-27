//! Adapters de transporte para `gerax-eda`.
//!
//! A crate core **não** depende de nenhum broker externo. Adapters
//! devem viver em crates separadas, evitando que Kafka, NATS,
//! RabbitMQ ou qualquer outra infraestrutura se tornem dependência
//! obrigatória do núcleo:
//!
//! ```text
//! gerax-eda             (core: bus, handlers, envelope)
//!     │
//!     ├── gerax-eda-rabbitmq   (implementado)
//!     ├── gerax-eda-kafka      (futuro)
//!     ├── gerax-eda-nats       (futuro)
//!     └── outros
//! ```
//!
//! Um adapter tipicamente:
//!
//! 1. implementa [`crate::EventBus`] (ou reutiliza
//!    [`crate::EventHandlerRegistry`] + [`crate::erased::ErasedEvent`]);
//! 2. traduz [`crate::EventMetadata`] para os headers do protocolo;
//! 3. respeita a mesma semântica de entrega documentada em
//!    [`crate::InMemoryEventBus`].
//!
//! O primeiro adapter concreto é
//! `gerax-eda-rabbitmq` ([Amqp/AMQP 0-9-1] via lapin). Ele **não**
//! implementa [`crate::EventBus`] porque serializar exige
//! `E: Serialize` (bound que a porta não expressa); expõe `publish`
//! inerente com o mesmo contrato e registra handlers por `event_type`.
//!
//! [Amqp/AMQP 0-9-1]: https://www.rabbitmq.com/amqp-0-9-1-reference
