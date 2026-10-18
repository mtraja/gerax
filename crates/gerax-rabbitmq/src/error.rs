//! Erros do adapter `gerax-rabbitmq`.

use thiserror::Error;

/// Erros produzidos pelo adapter `gerax-rabbitmq`.
///
/// Os erros são **tipados** por categoria (`§27` do skill): conexão,
/// canal, publicação, serialização, consumo e configuração. Internals
/// do [lapin] são convertidos — não expostos através da API de
/// [`gerax_eda`].
///
/// A conversão para [`gerax_eda::EventBusError`] é fornecida para que
/// um chmador que precise do contrato da porta possa fazer a ponte
/// sem conhecer os detalhes AMQP.
///
/// [lapin]: https://crates.io/crates/lapin
#[derive(Debug, Error)]
pub enum RabbitMqError {
    /// Falha de infraestrutura AMQP (conexão, canal, consumo, ...).
    #[error("connection error: {0}")]
    Connection(#[from] lapin::Error),

    /// Falha específica ao criação/uso de um canal.
    #[error("channel error: {0}")]
    Channel(String),

    /// Falha ao publicar uma mensagem.
    #[error("publish error: {0}")]
    Publish(String),

    /// O broker rejeitou a publicação (publisher confirm `nack`).
    #[error("broker rejected the publish (nack)")]
    PublisherNack,

    /// Falha ao serializar o envelope/payload.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Falha ao desserializar o envelope/payload recebido.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Falha no fluxo de consumo.
    #[error("consumer error: {0}")]
    Consumer(String),

    /// Configuração inválida/inconsistente.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// O envelope é inconsistente: `event_type` do envelope não condiz
    /// com [`Event::event_type`](gerax_eda::Event::event_type).
    #[error("invalid event: {0}")]
    InvalidEvent(String),
}

impl RabbitMqError {
    /// Constrói [`RabbitMqError::Configuration`].
    pub(crate) fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }
}

/// Conversão para o erro da porta [`gerax_eda::EventBus`] (ponte).
///
/// O impl da porta em [`crate::RabbitMqEventBus`] usa esta conversão
/// para mapear falhas de transporte para
/// [`gerax_eda::EventBusError::PublishFailed`]. Quando a aplicação
/// orquestra a transição `EventStore → outbox → broker`, a mesma
/// conversão mantém os erros de infraestrutura dentro do contrato
/// da porta.
impl From<RabbitMqError> for gerax_eda::EventBusError {
    fn from(value: RabbitMqError) -> Self {
        gerax_eda::EventBusError::PublishFailed(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erro_exibe_contexto_legivel() {
        let err = RabbitMqError::Configuration("uri is empty".into());
        assert_eq!(err.to_string(), "configuration error: uri is empty");
    }

    #[test]
    fn conversao_para_event_bus_error_cumpre_o_contrato_da_porta() {
        let err = RabbitMqError::PublisherNack;
        let bus_err: gerax_eda::EventBusError = err.into();

        assert!(matches!(
            bus_err,
            gerax_eda::EventBusError::PublishFailed(_)
        ));
        assert!(bus_err.to_string().contains("nack"));
    }

    #[test]
    fn invalid_event_exibe_detalhe() {
        let err = RabbitMqError::InvalidEvent("metadata vs evento diferem".into());
        assert!(err.to_string().contains("metadata vs evento"));
    }
}
