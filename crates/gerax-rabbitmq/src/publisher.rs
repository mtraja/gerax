//! Publicação de mensagens com publisher confirms (`§12`, `§13`).
//!
//! O [`RabbitMqEventBus`](crate::RabbitMqEventBus) delega a publicação
//! para o [`RabbitMqPublisher`]:
//!
//! ```text
//! EventEnvelope
//!       │
//!       ▼
//! MessageEnvelope ──serialize──► RabbitMQ message ──basic_publish──► Broker
//! ```
//!
//! Com publisher confirms habilitado (padrão), `basic_publish()` voltar
//! **não** significa mensagem persistida (`§13`): o resultado é
//! distinguido entre confirmado, rejeitado (`nack`) ou erro de
//! transporte/serialização.

use lapin::Channel;
use lapin::options::{BasicPublishOptions, ConfirmSelectOptions};
use lapin::types::ShortString;
use tracing::{debug, trace};

use crate::config::RabbitMqConfig;
use crate::connection::RabbitMqConnection;
use crate::envelope::{EnvelopeCodec, JsonEnvelopeCodec, MessageEnvelope, amqp_properties};
use crate::error::RabbitMqError;
use crate::topology;

/// Publica envelopes no exchange de eventos.
///
/// Mantém **um canal de longa duração** (criado uma vez) usado para
/// todas as publicações (`§24`/`§25`). O [lapin] `Channel` é seguro
/// para uso concorrente, logo o publisher pode ser compartilhado via
/// [`std::sync::Arc`].
pub struct RabbitMqPublisher<C: EnvelopeCodec = JsonEnvelopeCodec> {
    channel: Channel,
    config: RabbitMqConfig,
    codec: C,
}

impl RabbitMqPublisher<JsonEnvelopeCodec> {
    /// Cria um publicador com o codec JSON padrão sobre uma conexão
    /// reutilizada, declarando o exchange de eventos (idempotente).
    pub async fn connect(
        connection: &RabbitMqConnection,
        config: RabbitMqConfig,
    ) -> Result<Self, RabbitMqError> {
        Self::connect_with_codec(connection, config, JsonEnvelopeCodec).await
    }
}

impl<C: EnvelopeCodec> RabbitMqPublisher<C> {
    /// Cria um publicador com codec custom em uma conexão reutilizada.
    pub async fn connect_with_codec(
        connection: &RabbitMqConnection,
        config: RabbitMqConfig,
        codec: C,
    ) -> Result<Self, RabbitMqError> {
        let channel = connection.channel().await?;

        topology::declare_exchange(&channel, &config)
            .await
            .map_err(RabbitMqError::from)?;

        if config.publisher_confirms {
            channel
                .confirm_select(ConfirmSelectOptions::default())
                .await?;
        }

        debug!(
            exchange = %config.exchange,
            publisher_confirms = %config.publisher_confirms,
            "publisher channel ready",
        );

        Ok(Self {
            channel,
            config,
            codec,
        })
    }

    /// Canal de publicação (reutilizável pelo republish de retry do
    /// consumer).
    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    /// Configuração usada por este publicador.
    pub fn config(&self) -> &RabbitMqConfig {
        &self.config
    }

    /// Serializa e publica o envelope no exchange, roteando por
    /// `event_type` como routing key (`§9`).
    ///
    /// Com confirms, aguarda a resposta do broker:
    ///
    /// ```text
    /// Ack / NotRequested ──► Ok
    /// Nack                ──► RabbitMqError::PublisherNack
    /// erro de transporte  ──► RabbitMqError::Publish
    /// ```
    pub async fn publish(&self, envelope: &MessageEnvelope) -> Result<(), RabbitMqError> {
        let payload = self.codec.encode(envelope)?;
        let routing_key = envelope.event_type().to_string();

        trace!(
            event_type = %routing_key,
            event_id = %envelope.event_id().as_uuid(),
            exchange = %self.config.exchange,
            bytes = payload.len(),
            "publishing event",
        );

        let properties = amqp_properties(envelope, self.config.delivery_mode);

        let confirm = self
            .channel
            .basic_publish(
                self.config.exchange.clone().into(),
                routing_key.clone().into(),
                BasicPublishOptions::default(),
                &payload,
                properties,
            )
            .await
            .map_err(|source| RabbitMqError::Publish(format!("basic_publish failed: {source}")))?;

        if !self.config.publisher_confirms {
            return Ok(());
        }

        match confirm.await {
            Ok(lapin::Confirmation::Ack(_)) | Ok(lapin::Confirmation::NotRequested) => {
                debug!(
                    event_type = %routing_key,
                    event_id = %envelope.event_id().as_uuid(),
                    "publish confirmed by broker",
                );
                Ok(())
            }
            Ok(lapin::Confirmation::Nack(_)) => Err(RabbitMqError::PublisherNack),
            Err(source) => Err(RabbitMqError::Publish(format!(
                "publisher confirm failed: {source}"
            ))),
        }
    }

    /// Idempotente: declara (ou revalida) o exchange de eventos.
    #[allow(dead_code)]
    pub(crate) async fn ensure_exchange(&self) -> Result<(), RabbitMqError> {
        topology::declare_exchange(&self.channel, &self.config).await?;
        Ok(())
    }
}

/// Extrai o nome do exchange como [`ShortString`] (conveniência interna).
#[allow(dead_code)]
pub(crate) fn exchange_name(config: &RabbitMqConfig) -> ShortString {
    config.exchange.clone().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_name_deriva_da_config() {
        let config = RabbitMqConfig::default();
        assert_eq!(exchange_name(&config).to_string(), "gerax.events");
    }
}
