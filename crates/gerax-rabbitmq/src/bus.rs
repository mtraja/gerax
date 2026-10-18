//! `RabbitMqEventBus` — a porta [`EventBus`](gerax_eda::EventBus)
//! implementada com RabbitMQ.
//!
//! A porta `EventBus::publish` exige `E: Event + Clone + Serialize` —
//! o bound que qualquer broker precisa para levar o payload ao wire.
//! [`RabbitMqEventBus`] **implementa a porta** delegando ao mesmo
//! mecanismo dos métodos inerentes ([`MessageEnvelope`], codec,
//! publisher confirms) e convertendo [`RabbitMqError`] em
//! [`gerax_eda::EventBusError`] via [`From`].
//!
//! Semântica do impl da porta:
//!
//! - valida a consistência `metadata.event_type` ×
//!   [`Event::event_type`] → [`gerax_eda::EventBusError::InvalidEvent`];
//! - publica no exchange com *publisher confirm* — falha de
//!   transporte vira [`gerax_eda::EventBusError::PublishFailed`],
//!   nunca silêncio;
//! - **não há handlers locais**: `HandlerNotFound`, retry e
//!   middleware são semântica do `InMemoryEventBus`, não do broker
//!   (retry/DLQ no consumer — ver
//!   [`RabbitMqConsumer`](crate::RabbitMqConsumer)).
//!
//! O adaptador também expõe os métodos inerentes
//! [`Self::publish_event`], [`Self::publish_envelope`] e
//! [`Self::publish_message`] com o erro tipado `RabbitMqError` —
//! para o chamador que prefere o erro do adapter ao erro da porta.

use async_trait::async_trait;
use gerax_eda::{Event, EventEnvelope, EventBus, EventBusError};
use serde::Serialize;

use crate::config::RabbitMqConfig;
use crate::connection::RabbitMqConnection;
use crate::envelope::{EnvelopeCodec, JsonEnvelopeCodec, MessageEnvelope};
use crate::error::RabbitMqError;
use crate::publisher::RabbitMqPublisher;

/// Bus de eventos `gerax-eda` transportado por RabbitMQ.
///
/// Agrega um [`RabbitMqPublisher`] e é seguro para uso assíncrono e
/// concorrente (`§26`): o [lapin] `Channel` compartilhado serializa os
/// frames AMQP, e cada chamada aguarda seu próprio publisher confirm.
pub struct RabbitMqEventBus<C: EnvelopeCodec = JsonEnvelopeCodec> {
    publisher: RabbitMqPublisher<C>,
}

impl RabbitMqEventBus<JsonEnvelopeCodec> {
    /// Abre uma conexão nova e cria o bus com o codec JSON padrão.
    ///
    /// Se o chamador já possui uma [`RabbitMqConnection`] compartilhada
    /// (ex.: com consumidores), prefira [`Self::from_connection`].
    pub async fn connect(config: RabbitMqConfig) -> Result<Self, RabbitMqError> {
        let connection = RabbitMqConnection::connect(config.clone()).await?;
        Self::from_connection(&connection, config).await
    }

    /// Cria o bus sobre uma conexão já estabelecida (reutilizada
    /// também por consumidores — `§24`).
    pub async fn from_connection(
        connection: &RabbitMqConnection,
        config: RabbitMqConfig,
    ) -> Result<Self, RabbitMqError> {
        let publisher =
            RabbitMqPublisher::connect_with_codec(connection, config, JsonEnvelopeCodec).await?;
        Ok(Self { publisher })
    }
}

impl<C: EnvelopeCodec> RabbitMqEventBus<C> {
    /// Abre uma conexão nova e cria o bus com codec custom.
    pub async fn connect_with_codec(
        config: RabbitMqConfig,
        codec: C,
    ) -> Result<Self, RabbitMqError> {
        let connection = RabbitMqConnection::connect(config.clone()).await?;
        Self::from_connection_with_codec(&connection, config, codec).await
    }

    /// Cria o bus sobre uma conexão existente com codec custom.
    pub async fn from_connection_with_codec(
        connection: &RabbitMqConnection,
        config: RabbitMqConfig,
        codec: C,
    ) -> Result<Self, RabbitMqError> {
        let publisher = RabbitMqPublisher::connect_with_codec(connection, config, codec).await?;
        Ok(Self { publisher })
    }

    /// Publica um evento tipado em um envelope novo
    /// ([`EventEnvelope::new`]) roteado por `event_type`.
    ///
    /// Conveniência sobre [`Self::publish_envelope`] com o erro do
    /// adapter; quando o chamador é genérico em
    /// [`EventBus`](gerax_eda::EventBus), use a porta.
    pub async fn publish_event<E>(&self, event: E) -> Result<(), RabbitMqError>
    where
        E: Event + Serialize,
    {
        self.publish_envelope(EventEnvelope::new(event)).await
    }

    /// Publica um envelope já construído (permite preservar id,
    /// correlation/causation e headers do fluxo de processamento).
    pub async fn publish_envelope<E>(&self, envelope: EventEnvelope<E>) -> Result<(), RabbitMqError>
    where
        E: Event + Serialize,
    {
        let message = MessageEnvelope::from_event(&envelope)?;
        self.publisher.publish(&message).await
    }

    /// Publica um envelope de transporte pronto.
    ///
    /// Usado por integrações de *Outbox* (`§23`), que já possuem um
    /// [`MessageEnvelope`] derivado de um [`gerax_es::StoredEvent`].
    pub async fn publish_message(&self, message: &MessageEnvelope) -> Result<(), RabbitMqError> {
        self.publisher.publish(message).await
    }

    /// Publicador interno (para cenários de baixo nível).
    pub fn publisher(&self) -> &RabbitMqPublisher<C> {
        &self.publisher
    }
}

#[async_trait]
impl<C: EnvelopeCodec> EventBus for RabbitMqEventBus<C> {
    async fn publish<E>(&self, envelope: EventEnvelope<E>) -> Result<(), EventBusError>
    where
        E: Event + Clone + Serialize,
    {
        if envelope.metadata.event_type != envelope.event.event_type() {
            return Err(EventBusError::InvalidEvent(format!(
                "metadata.event_type `{}` does not match event.event_type `{}`",
                envelope.metadata.event_type,
                envelope.event.event_type(),
            )));
        }
        self.publish_envelope(envelope).await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requer_event_bus<C: EnvelopeCodec>()
    where
        RabbitMqEventBus<C>: EventBus,
    {
    }

    #[test]
    fn rabbit_mq_event_bus_satisfaz_a_porta_event_bus() {
        requer_event_bus::<JsonEnvelopeCodec>();
    }
}
