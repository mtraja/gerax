//! `RabbitMqEventBus` — a porta `EventBus` implementada com RabbitMQ.
//!
//! # Por que `gerax_eda::EventBus` não é implementada?
//!
//! O contribuinte de eventos da porta exige apenas `E: Event`:
//!
//! ```rust,ignore
//! async fn publish<E>(&self, event: E) -> Result<(), EventBusError>
//! where E: Event;
//! ```
//!
//! Publicar em RabbitMQ demanda serializar o **payload** do evento
//! (`E: Serialize`) e, no consumo, desserializá-lo de volta
//! (`E: DeserializeOwned`) — bounds que a porta não expressa. O adapter
//! **não enfraquece nem aumenta** o contrato: oferece o mesmo
//! comportamento com a assinatura precisa (`E: Event + Serialize`) na
//! forma inerente [`Self::publish`]. Documentado também em
//! `gerax-eda/src/adapters/mod.rs`.
//!
//! `RabbitMqError` converte para [`gerax_eda::EventBusError`]
//! ([`From`]), então a aplicação pode orquestrar
//! `EventStore → outbox → broker` mantendo o erro dentro do contrato
//! da porta (`§22`/`§23`).

use gerax_eda::{Event, EventEnvelope};
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
    pub async fn publish<E>(&self, event: E) -> Result<(), RabbitMqError>
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
