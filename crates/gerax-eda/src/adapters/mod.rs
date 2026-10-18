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
//!     ├── gerax-eda-rabbitmq      (AMQP 0-9-1)
//!     ├── gerax-rabbitmq          (AMQP 0-9-1: envelope, retry, DLQ)
//!     ├── gerax-rabbitmq-stream   (RabbitMQ Streams)
//!     ├── gerax-eda-kafka         (futuro)
//!     ├── gerax-eda-nats          (futuro)
//!     └── outros
//! ```
//!
//! Um adapter tipicamente:
//!
//! 1. implementa [`crate::EventBus`]: o bound
//!    `E: Event + Clone + Serialize` do `publish` da porta é
//!    exatamente o que qualquer broker precisa para levar o payload
//!    ao wire — serializar é responsabilidade do adapter;
//! 2. valida a consistência do envelope
//!    ([`crate::EventBusError::InvalidEvent`]) e converte o erro de
//!    transporte em [`crate::EventBusError::PublishFailed`];
//! 3. traduz [`crate::EventMetadata`] para os headers do protocolo;
//! 4. documenta a semântica de **entrega** específica (publisher
//!    confirms, ack, retry, DLQ) — o dispatch local documentado em
//!    [`crate::InMemoryEventBus`] (handlers, `HandlerNotFound`,
//!    middleware) não se aplica a um broker.
//!
//! O lado de **assinatura** permanece específico de cada adapter
//! (registro de handlers por `event_type`, política de ack): a porta
//! unifica o **publish**.
//!
//! [Amqp/AMQP 0-9-1]: https://www.rabbitmq.com/amqp-0-9-1-reference
