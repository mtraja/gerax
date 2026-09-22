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
//!     ├── gerax-eda-kafka      (futuro)
//!     ├── gerax-eda-nats       (futuro)
//!     ├── gerax-eda-rabbitmq   (futuro)
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
//! Nesta primeira versão não há adapters externos — apenas a
//! estrutura preparada para recebê-los (ver também
//! [`crate::EventBusPublisher`], ponto de evolução para Outbox).
