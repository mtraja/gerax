//! Publisher de eventos de integração sobre NATS JetStream.
//!
//! Implementa a porta [`IntegrationPublisher`] da `gerax-eda` usando o
//! [`NatsPublisher`] existente. Em modo JetStream a publicação só
//! retorna `Ok` depois do `PubAck` do servidor — condição necessária
//! para o Event Relay confirmar o evento na origem.
//!
//! ```text
//! KurrentDB ──► gerax-eda relay ──► NatsIntegrationPublisher ──► JetStream
//!                                        │
//!                                   PubAck confirmado
//! ```
//!
//! A entrega continua sendo **at-least-once**: `Nats-Msg-Id` (o
//! `event_id` original) ativa a deduplicação do JetStream dentro da
//! janela configurada, mas não substitui a idempotência do consumidor.

use async_trait::async_trait;

use gerax_eda::event_relay::{IntegrationEvent, IntegrationPublisher, PublishError};

use crate::client::NatsClient;
use crate::envelope::NatsEnvelope;
use crate::error::NatsError;
use crate::publisher::NatsPublisher;

/// Publisher de [`IntegrationEvent`]s no NATS.
#[derive(Debug, Clone)]
pub struct NatsIntegrationPublisher {
    publisher: NatsPublisher,
}

impl NatsIntegrationPublisher {
    /// Cria o publisher a partir de um [`NatsPublisher`] configurado.
    pub fn new(publisher: NatsPublisher) -> Self {
        Self { publisher }
    }

    /// Cria o publisher a partir de uma conexão [`NatsClient`].
    pub fn from_client(client: &NatsClient) -> Self {
        Self::new(NatsPublisher::new(client))
    }

    /// Publisher NATS subjacente.
    pub fn publisher(&self) -> &NatsPublisher {
        &self.publisher
    }

    /// Consome o adaptador e devolve o [`NatsPublisher`].
    pub fn into_publisher(self) -> NatsPublisher {
        self.publisher
    }
}

impl From<NatsPublisher> for NatsIntegrationPublisher {
    fn from(publisher: NatsPublisher) -> Self {
        Self::new(publisher)
    }
}

#[async_trait]
impl IntegrationPublisher for NatsIntegrationPublisher {
    async fn publish(&self, event: &IntegrationEvent) -> Result<(), PublishError> {
        event.validate().map_err(|error| {
            PublishError::permanent(format!("invalid integration event: {error}"))
        })?;

        let envelope = NatsEnvelope::from_integration_event(event);
        self.publisher
            .publish_envelope(&envelope)
            .await
            .map_err(PublishError::from)
    }
}

/// Classifica um [`NatsError`] para a política de retry do relay.
///
/// Falhas de transporte/servidor (conexão, publish, JetStream,
/// timeout) são transitórias. Falhas de contrato/configuração
/// (subject inválido, envelope inválido, serialização) são
/// permanentes e devem ir para park.
impl From<NatsError> for PublishError {
    fn from(error: NatsError) -> Self {
        match error {
            NatsError::Connection(_)
            | NatsError::Publish(_)
            | NatsError::Subscription(_)
            | NatsError::Request(_)
            | NatsError::JetStream(_)
            | NatsError::Timeout
            | NatsError::Handler(_) => PublishError::Retryable(error.to_string()),
            NatsError::Serialization(_)
            | NatsError::Deserialization(_)
            | NatsError::Configuration(_)
            | NatsError::InvalidSubject(_)
            | NatsError::InvalidEvent(_) => PublishError::Permanent(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;
    use gerax_es::EventId;

    fn event() -> IntegrationEvent {
        IntegrationEvent::new(
            EventId::new(),
            "aluno.matriculado",
            "aluno-1",
            "Aluno",
            0,
            Utc::now(),
            serde_json::json!({ "aluno_id": "aluno-1" }),
        )
    }

    #[test]
    fn falhas_de_transporte_sao_retentaveis() {
        for error in [
            NatsError::Connection("offline".into()),
            NatsError::Publish("nack".into()),
            NatsError::Subscription("lost".into()),
            NatsError::Request("timeout".into()),
            NatsError::JetStream("unavailable".into()),
            NatsError::Timeout,
            NatsError::Handler("handler".into()),
        ] {
            assert!(
                PublishError::from(error).is_retryable(),
                "deveria ser retentável"
            );
        }
    }

    #[test]
    fn falhas_de_contrato_sao_permanentes() {
        for error in [
            NatsError::Serialization("payload".into()),
            NatsError::Deserialization("envelope".into()),
            NatsError::Configuration("subject".into()),
            NatsError::InvalidSubject("wildcard".into()),
            NatsError::InvalidEvent("tipo".into()),
        ] {
            assert!(
                PublishError::from(error).is_permanent(),
                "deveria ser permanente"
            );
        }
    }

    #[test]
    fn envelope_de_integracao_preserva_event_id() {
        let event = event();
        let envelope = NatsEnvelope::from_integration_event(&event);

        assert_eq!(envelope.event_id(), event.id);
        assert_eq!(envelope.stream_revision, Some(0));
    }
}
