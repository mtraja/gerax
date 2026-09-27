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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erro_exibe_contexto() {
        let err = RabbitMqError::InvalidEvent("metadata vs evento diferem".into());
        assert!(err.to_string().contains("metadata vs evento"));
    }
}
