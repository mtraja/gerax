//! Erros do adapter RabbitMQ.

use thiserror::Error;

/// Erros produzidos pelo adapter `gerax-eda-rabbitmq`.
#[derive(Debug, Error)]
pub enum RabbitMqError {
    /// Falha de infraestrutura AMQP (conexão, canal, consumo, ...).
    #[error("amqp error: {0}")]
    Amqp(#[from] lapin::Error),

    /// Falha de codec (serialização/desserialização do payload).
    #[error("codec error: {0}")]
    Codec(String),

    /// O broker rejeitou a publicação (publisher confirm `nack`).
    #[error("broker rejected the publish (nack)")]
    PublisherNack,

    /// O envelope é inconsistente: `metadata.event_type` não condiz
    /// com [`Event::event_type`](gerax_eda::Event::event_type).
    #[error("invalid event: {0}")]
    InvalidEvent(String),
}

/// Conversão para o erro da porta [`gerax_eda::EventBus`] (ponte).
///
/// O impl da porta em [`crate::RabbitMqEventBus`] usa esta conversão
/// para mapear falhas de transporte em
/// [`gerax_eda::EventBusError::PublishFailed`].
impl From<RabbitMqError> for gerax_eda::EventBusError {
    fn from(value: RabbitMqError) -> Self {
        gerax_eda::EventBusError::PublishFailed(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erro_exibe_contexto() {
        let err = RabbitMqError::InvalidEvent("metadata vs evento diferem".into());
        assert!(err.to_string().contains("metadata vs evento"));
    }

    #[test]
    fn conversao_para_event_bus_error_e_publish_failed() {
        let err = RabbitMqError::PublisherNack;
        let bus_err: gerax_eda::EventBusError = err.into();
        assert!(matches!(
            bus_err,
            gerax_eda::EventBusError::PublishFailed(_)
        ));
    }
}
