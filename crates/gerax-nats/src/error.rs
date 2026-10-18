//! Erros do adapter `gerax-nats` (`§22`).
//!
//! Os erros são **tipados** por categoria: conexão, publicação,
//! assinatura, request/reply, JetStream, serialização e configuração.
//! Internals do [async-nats] são convertidos — não expostos através da
//! API pública de [`gerax_eda`].
//!
//! [async-nats]: https://crates.io/crates/async-nats

use thiserror::Error;

/// Erros produzidos pelo adapter `gerax-nats`.
#[derive(Debug, Error)]
pub enum NatsError {
    /// Falha de infraestrutura (conexão, reconexão, flush).
    #[error("connection error: {0}")]
    Connection(String),

    /// Falha ao publicar uma mensagem.
    #[error("publish error: {0}")]
    Publish(String),

    /// Falha ao criar/usar uma subscription.
    #[error("subscription error: {0}")]
    Subscription(String),

    /// Falha em request/reply.
    #[error("request error: {0}")]
    Request(String),

    /// Falha em operação JetStream (stream, consumer, PubAck).
    #[error("jetstream error: {0}")]
    JetStream(String),

    /// Falha ao serializar envelope/payload.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Falha ao desserializar envelope/payload recebido.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Configuração inválida/inconsistente.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// Subject fora da política do Gerax (token vazio, wildcard ilegal…).
    #[error("invalid subject: {0}")]
    InvalidSubject(String),

    /// O envelope é inconsistente: `event_type` não condiz com
    /// [`Event::event_type`](gerax_eda::Event::event_type).
    #[error("invalid event: {0}")]
    InvalidEvent(String),

    /// Operação excedeu o timeout configurado.
    #[error("timeout")]
    Timeout,

    /// Falha de processamento reportada pelo handler do consumidor.
    #[error("handler error: {0}")]
    Handler(String),
}

impl NatsError {
    /// Constrói [`NatsError::Configuration`].
    pub(crate) fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Constrói [`NatsError::InvalidSubject`].
    pub(crate) fn invalid_subject(message: impl Into<String>) -> Self {
        Self::InvalidSubject(message.into())
    }
}

impl From<serde_json::Error> for NatsError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization(source.to_string())
    }
}

/// Conversão para o erro da porta [`gerax_eda::EventBus`] (ponte).
///
/// [`NatsError::InvalidEvent`] preserva a variante da porta; as demais
/// falhas de transporte viram
/// [`gerax_eda::EventBusError::PublishFailed`].
impl From<NatsError> for gerax_eda::EventBusError {
    fn from(value: NatsError) -> Self {
        match value {
            NatsError::InvalidEvent(message) => gerax_eda::EventBusError::InvalidEvent(message),
            other => gerax_eda::EventBusError::PublishFailed(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erro_exibe_contexto_legivel() {
        let err = NatsError::Configuration("url is empty".into());
        assert_eq!(err.to_string(), "configuration error: url is empty");
    }

    #[test]
    fn timeout_exibe_mensagem_curta() {
        assert_eq!(NatsError::Timeout.to_string(), "timeout");
    }

    #[test]
    fn conversao_para_event_bus_error_preserva_invalid_event() {
        let bus_err: gerax_eda::EventBusError = NatsError::InvalidEvent("x vs y".into()).into();
        assert!(matches!(bus_err, gerax_eda::EventBusError::InvalidEvent(_)));
    }

    #[test]
    fn conversao_para_event_bus_error_mapeia_transportes_para_publish_failed() {
        let bus_err: gerax_eda::EventBusError = NatsError::Timeout.into();
        assert!(matches!(
            bus_err,
            gerax_eda::EventBusError::PublishFailed(_)
        ));
    }

    #[test]
    fn serde_json_vira_serialization() {
        let err: NatsError = serde_json::from_str::<serde_json::Value>("{")
            .unwrap_err()
            .into();
        assert!(matches!(err, NatsError::Serialization(_)));
    }
}
