//! `NatsEventBus` — a porta [`EventBus`](gerax_eda::EventBus)
//! implementada com NATS.
//!
//! A porta `EventBus::publish` exige `E: Event + Clone + Serialize` —
//! o bound que qualquer broker precisa para levar o payload ao wire.
//! [`NatsEventBus`] **implementa a porta** delegando ao mesmo mecanismo
//! dos métodos inerentes ([`NatsEnvelope`], codec JSON, PubAck/flush) e
//! convertendo [`NatsError`] em [`gerax_eda::EventBusError`] via
//! [`From`].
//!
//! Semântica do impl da porta:
//!
//! - valida a consistência `metadata.event_type` ×
//!   [`Event::event_type`] → [`gerax_eda::EventBusError::InvalidEvent`];
//! - publica no subject `<prefixo>.<event_type>` — JetStream aguarda o
//!   `PubAck` (persistência confirmada), Core NATS faz `flush`; falha
//!   de transporte vira [`gerax_eda::EventBusError::PublishFailed`],
//!   nunca silêncio;
//! - **não há handlers locais**: `HandlerNotFound`, retry e middleware
//!   são semântica do `InMemoryEventBus`, não do broker (retry/DLQ no
//!   consumer — ver [`NatsConsumer`](crate::NatsConsumer)).
//!
//! O adaptador também expõe os métodos inerentes
//! [`Self::publish_event`] e [`Self::publish_envelope`] com o erro
//! tipado `NatsError` — para o chamador que prefere o erro do adapter
//! ao erro da porta.

use async_trait::async_trait;
use gerax_eda::{Event, EventBus, EventBusError, EventEnvelope};
use serde::Serialize;

use crate::client::NatsClient;
use crate::config::NatsConfig;
use crate::envelope::NatsEnvelope;
use crate::error::NatsError;
use crate::publisher::NatsPublisher;

/// Bus de eventos `gerax-eda` transportado por NATS.
///
/// Seguro para uso assíncrono e concorrente: o client async-nats
/// multiplexa a conexão, e cada publicação JetStream aguarda o seu
/// `PubAck`.
pub struct NatsEventBus {
    publisher: NatsPublisher,
}

impl NatsEventBus {
    /// Conecta ao servidor e cria o bus com a configuração fornecida.
    pub async fn connect(config: NatsConfig) -> Result<Self, NatsError> {
        let client = NatsClient::connect(config).await?;
        Ok(Self::from_client(&client))
    }

    /// Cria o bus sobre uma conexão já estabelecida (reutilizada também
    /// por consumidores — `§12`).
    pub fn from_client(client: &NatsClient) -> Self {
        Self {
            publisher: NatsPublisher::new(client),
        }
    }

    /// Publica um evento tipado em um envelope novo
    /// ([`EventEnvelope::new`]) no subject `<prefixo>.<event_type>`.
    ///
    /// Conveniência sobre [`Self::publish_envelope`] com o erro do
    /// adapter; quando o chamador é genérico em
    /// [`EventBus`](gerax_eda::EventBus), use a porta.
    pub async fn publish_event<E>(&self, event: E) -> Result<(), NatsError>
    where
        E: Event + Serialize,
    {
        self.publish_envelope(EventEnvelope::new(event)).await
    }

    /// Publica um envelope já construído (permite preservar id,
    /// correlation/causation e headers do fluxo de processamento).
    pub async fn publish_envelope<E>(&self, envelope: EventEnvelope<E>) -> Result<(), NatsError>
    where
        E: Event + Serialize,
    {
        let transport = NatsEnvelope::from_event(&envelope)?;
        self.publisher.publish_envelope(&transport).await
    }

    /// Publicador interno (para cenários de baixo nível, ex.: outbox).
    pub fn publisher(&self) -> &NatsPublisher {
        &self.publisher
    }
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish<E>(&self, envelope: EventEnvelope<E>) -> Result<(), EventBusError>
    where
        E: Event + Clone + Serialize,
    {
        validate_consistency(&envelope)?;
        self.publish_envelope(envelope).await.map_err(Into::into)
    }
}

/// Contrato da porta (`§16`): o envelope precisa ser consistente com o
/// evento que carrega — testável sem conexão.
fn validate_consistency<E: Event>(envelope: &EventEnvelope<E>) -> Result<(), EventBusError> {
    if envelope.metadata.event_type != envelope.event.event_type() {
        return Err(EventBusError::InvalidEvent(format!(
            "metadata.event_type `{}` does not match event.event_type `{}`",
            envelope.metadata.event_type,
            envelope.event.event_type(),
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    fn requer_event_bus()
    where
        NatsEventBus: EventBus,
    {
    }

    #[test]
    fn nats_event_bus_satisfaz_a_porta_event_bus() {
        requer_event_bus();
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[tokio::test]
    async fn porta_rejeita_envelope_inconsistente_antes_do_publish() {
        let mut envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        });
        envelope.metadata.event_type = "outro.evento".into();

        let err = validate_consistency(&envelope).unwrap_err();
        assert!(matches!(err, EventBusError::InvalidEvent(_)));
        assert!(err.to_string().contains("outro.evento"));
    }

    #[tokio::test]
    async fn porta_aceita_envelope_consistente() {
        let envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        });
        assert!(validate_consistency(&envelope).is_ok());
    }
}
