//! Produtor: `RabbitMqEventBus`.
//!
//! Um producer funcional para eventos `gerax-eda` sobre RabbitMQ.

use std::sync::Arc;

use gerax_eda::{Event, EventEnvelope};
use lapin::options::{BasicPublishOptions, ConfirmSelectOptions, ExchangeDeclareOptions};
use lapin::{Channel, Confirmation, Connection, ConnectionProperties};
use serde::Serialize;
use tracing::debug;

use crate::codec::{EventCodec, JsonEventCodec};
use crate::config::RabbitMqConfig;
use crate::error::RabbitMqError;
use crate::map::properties_from_envelope;

/// Publica envelopes em um exchange RabbitMQ.
///
/// O adapter **não** implementa [`EventBus`](gerax_eda::EventBus):
/// serializar exige `E: Serialize`, um bound que a porta `EventBus`
/// não conhece (ver [`crate::EventCodec`]). Em troca, `publish` é um
/// método inerente com exatamente o mesmo contrato.
///
/// # Confirm
///
/// O canal usa **publisher confirm**: `publish` apenas retorna depois
/// que o broker confirma a persistência da mensagem
/// ([`Channel::confirm_select`](lapin::Channel::confirm_select) + Ack).
#[derive(Clone)]
pub struct RabbitMqEventBus<C: EventCodec = JsonEventCodec> {
    /// Mantém a conexão viva pelo tempo de vida do bus (canal não a
    /// segura; descartá-la fecharia a conexão).
    #[allow(dead_code)]
    connection: Arc<Connection>,
    channel: Channel,
    config: RabbitMqConfig,
    codec: C,
}

impl<C: EventCodec> RabbitMqEventBus<C> {
    /// Conecta, declara o exchange com os parâmetros da configuração e
    /// habilita publisher confirm.
    pub async fn connect(config: RabbitMqConfig, codec: C) -> Result<Self, RabbitMqError> {
        let connection = Connection::connect(
            &config.uri,
            ConnectionProperties::default().enable_auto_recover(),
        )
        .await?;
        let channel = connection.create_channel().await?;
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await?;
        channel
            .exchange_declare(
                config.exchange.as_str().into(),
                config.exchange_kind.clone(),
                ExchangeDeclareOptions {
                    durable: config.durable,
                    ..Default::default()
                },
                Default::default(),
            )
            .await?;

        Ok(Self {
            connection: Arc::new(connection),
            channel,
            config,
            codec,
        })
    }

    /// Referência à configuração usada na conexão.
    pub fn config(&self) -> &RabbitMqConfig {
        &self.config
    }

    /// Publica um envelope no exchange.
    ///
    /// - `routing_key` = `metadata.event_type` (tópicos);
    /// - propriedades mapeadas conforme [`crate::map`];
    /// - corpo = payload serializado pelo codec.
    ///
    /// Falha com [`RabbitMqError::InvalidEvent`] quando o código do
    /// evento não condiz com `metadata.event_type`.
    pub async fn publish<E>(&self, envelope: &EventEnvelope<E>) -> Result<(), RabbitMqError>
    where
        E: Event + Serialize,
    {
        if envelope.metadata.event_type != envelope.event.event_type() {
            return Err(RabbitMqError::InvalidEvent(format!(
                "metadata.event_type `{}` differs from Event::event_type() `{}`",
                envelope.metadata.event_type,
                envelope.event.event_type(),
            )));
        }

        let payload = self.codec.encode_payload(&envelope.event)?;
        let properties = properties_from_envelope(envelope);

        let confirm = self
            .channel
            .basic_publish(
                self.config.exchange.as_str().into(),
                envelope.metadata.event_type.as_str().into(),
                BasicPublishOptions::default(),
                &payload,
                properties,
            )
            .await?;

        match confirm.await {
            Ok(Confirmation::Ack(_)) => {
                debug!(
                    event_type = %envelope.metadata.event_type,
                    event_id = %envelope.id.as_uuid(),
                    exchange = %self.config.exchange,
                    "event published",
                );
                Ok(())
            }
            Ok(Confirmation::NotRequested) => Ok(()),
            Ok(Confirmation::Nack(_)) => Err(RabbitMqError::PublisherNack),
            Err(source) => Err(source.into()),
        }
    }

    /// Cria um novo envelope (novo `EventId`, `occurred_at = agora`)
    /// e o publica, devolvendo-o para o chamador.
    pub async fn publish_new<E>(&self, event: E) -> Result<EventEnvelope<E>, RabbitMqError>
    where
        E: Event + Serialize,
    {
        let envelope = EventEnvelope::new(event);
        self.publish(&envelope).await?;
        Ok(envelope)
    }
}
