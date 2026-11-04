//! Porta de publicação do Event Relay.
//!
//! A `gerax-eda` define a abstração; cada adapter de transporte
//! (por exemplo, `gerax-nats`) implementa [`IntegrationPublisher`].
//! Nenhum detalhe de protocolo (subject, JetStream, PubAck) atravessa
//! este contrato.
//!
//! O relay só confirma o evento na origem **depois** que
//! [`IntegrationPublisher::publish`] retorna `Ok`. Em caso de falha,
//! a origem deve reentregar o evento.

use async_trait::async_trait;
use thiserror::Error;

use super::envelope::IntegrationEvent;
use super::handler::{HandlerError, IntegrationEventHandler};

/// Erro de publicação, classificado para a política de retry do relay.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PublishError {
    /// Falha transitória: a publicação pode ser repetida.
    #[error("retryable publish error: {0}")]
    Retryable(String),

    /// Falha definitiva: repetir a publicação não resolve.
    #[error("permanent publish error: {0}")]
    Permanent(String),
}

impl PublishError {
    /// Cria um erro transitório.
    pub fn retryable(message: impl Into<String>) -> Self {
        Self::Retryable(message.into())
    }

    /// Cria um erro definitivo.
    pub fn permanent(message: impl Into<String>) -> Self {
        Self::Permanent(message.into())
    }

    /// `true` quando a publicação pode ser repetida.
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }

    /// `true` quando a falha é definitiva.
    pub const fn is_permanent(&self) -> bool {
        matches!(self, Self::Permanent(_))
    }
}

/// Porta de publicação de eventos de integração.
///
/// Implementações devem aguardar a **confirmação real** do transporte
/// antes de retornar `Ok` — por exemplo, o `PubAck` do JetStream. Um
/// `Ok` prematuro faria o relay confirmar no KurrentDB uma mensagem
/// que nunca foi persistida.
#[async_trait]
pub trait IntegrationPublisher: Send + Sync {
    /// Publica o evento, aguardando a confirmação do transporte.
    async fn publish(&self, event: &IntegrationEvent) -> Result<(), PublishError>;
}

/// Adaptador de [`IntegrationPublisher`] para [`IntegrationEventHandler`].
///
/// Permite registrar um publisher no [`EventDispatcher`](super::EventDispatcher)
/// sem que o relay conheça o transporte.
pub struct PublisherHandler<P> {
    publisher: P,
}

impl<P> PublisherHandler<P> {
    /// Cria o adaptador em torno de um publisher.
    pub fn new(publisher: P) -> Self {
        Self { publisher }
    }

    /// Publisher subjacente.
    pub fn publisher(&self) -> &P {
        &self.publisher
    }

    /// Consome o adaptador e devolve o publisher.
    pub fn into_publisher(self) -> P {
        self.publisher
    }
}

#[async_trait]
impl<P> IntegrationEventHandler for PublisherHandler<P>
where
    P: IntegrationPublisher,
{
    async fn handle(&self, event: &IntegrationEvent) -> Result<(), HandlerError> {
        self.publisher.publish(event).await.map_err(|error| {
            if error.is_retryable() {
                HandlerError::Retryable(error.to_string())
            } else {
                HandlerError::Permanent(error.to_string())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use chrono::Utc;
    use gerax_es::EventId;

    fn event() -> IntegrationEvent {
        IntegrationEvent::new(
            EventId::new(),
            "AlunoMatriculado",
            "aluno-1",
            "Aluno",
            0,
            Utc::now(),
            serde_json::json!({ "aluno_id": "aluno-1" }),
        )
    }

    struct FixedPublisher {
        result: Result<(), PublishError>,
    }

    #[async_trait]
    impl IntegrationPublisher for FixedPublisher {
        async fn publish(&self, _event: &IntegrationEvent) -> Result<(), PublishError> {
            self.result.clone()
        }
    }

    #[test]
    fn classificacao_de_erros_e_estavel() {
        assert!(PublishError::retryable("offline").is_retryable());
        assert!(!PublishError::retryable("offline").is_permanent());
        assert!(PublishError::permanent("invalid").is_permanent());
        assert!(!PublishError::permanent("invalid").is_retryable());
    }

    #[tokio::test]
    async fn publisher_handler_propaga_sucesso() {
        let handler = PublisherHandler::new(FixedPublisher { result: Ok(()) });
        assert!(handler.handle(&event()).await.is_ok());
    }

    #[tokio::test]
    async fn publisher_handler_mapeia_falhas() {
        let retry = PublisherHandler::new(FixedPublisher {
            result: Err(PublishError::retryable("offline")),
        });
        let error = retry.handle(&event()).await.unwrap_err();
        assert!(error.is_retryable());

        let permanent = PublisherHandler::new(FixedPublisher {
            result: Err(PublishError::permanent("invalid")),
        });
        let error = permanent.handle(&event()).await.unwrap_err();
        assert!(error.is_permanent());
    }

    #[test]
    fn publisher_trait_e_objetivavel() {
        let publisher: Arc<dyn IntegrationPublisher> = Arc::new(FixedPublisher { result: Ok(()) });
        assert!(Arc::strong_count(&publisher) == 1);
    }
}
