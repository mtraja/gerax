//! Erros do adapter `gerax-rabbitmq-stream`.
//!
//! Os erros são **tipados por categoria** (§22), nunca por tipo do
//! cliente: nenhum item de `rabbitmq_stream_client` vaza para a API
//! de [`gerax_eda`]. Um chamador que só conhece a porta do
//! `gerax-eda` continua vendo [`gerax_eda::EventBusError`].
//!
//! ```text
//! rabbitmq_stream_client::ClientError            ─┐
//! rabbitmq_stream_client::StreamCreateError        │ From<...>
//! rabbitmq_stream_client::ProducerCreateError      ├─► RabbitMqStreamError
//! rabbitmq_stream_client::ProducerPublishError     │
//! rabbitmq_stream_client::ProducerCloseError       │
//! rabbitmq_stream_client::ConsumerCreateError      │
//! rabbitmq_stream_client::ConsumerCloseError       │
//! rabbitmq_stream_client::ConsumerStoreOffsetError ─┘
//! ```

use thiserror::Error;

use rabbitmq_stream_client::error::{
    ClientError, ConsumerCloseError, ConsumerCreateError, ConsumerStoreOffsetError,
    ProducerCloseError, ProducerCreateError, ProducerPublishError, StreamCreateError,
    StreamDeleteError,
};

/// Erros produzidos pelo adapter `gerax-rabbitmq-stream`.
///
/// As categorias seguem o ciclo de vida da mensagem: conexão →
/// stream → publicação → consumo → offset → assinatura. Erros de
/// serialização e configuração são transversais.
#[derive(Debug, Error)]
pub enum RabbitMqStreamError {
    /// Falha de infraestrutura ao falar com o broker (socket, TLS,
    /// autenticação, protocolo).
    #[error("connection error: {0}")]
    Connection(#[from] ClientError),

    /// Falha ao criar/verificar o stream de eventos.
    #[error("stream error: {0}")]
    Stream(String),

    /// O stream informado já existe (não é falha: §7 — criação
    /// idempotente).
    #[error("stream already exists: {0}")]
    StreamAlreadyExists(String),

    /// Falha ao criar o producer.
    #[error("producer error: {0}")]
    Producer(String),

    /// Falha ao publicar uma mensagem.
    #[error("producer publish error: {0}")]
    Publish(String),

    /// O broker não confirmou a publicação.
    #[error("publish not confirmed by the broker")]
    PublishNotConfirmed,

    /// O producer já foi fechado e não aceita novas publicações.
    #[error("producer is closed")]
    ProducerClosed,

    /// Falha ao criar o consumer.
    #[error("consumer error: {0}")]
    Consumer(String),

    /// Falha ao fechar producer/consumer.
    #[error("close error: {0}")]
    Close(String),

    /// Falha ao serializar/desserializar o payload do evento.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Falha ao ler/gravar o offset do consumidor.
    #[error("offset error: {0}")]
    Offset(String),

    /// Falha no lifecycle da assinatura (start/stop).
    #[error("subscription error: {0}")]
    Subscription(String),

    /// Configuração inválida ou inconsistente.
    #[error("configuration error: {0}")]
    Configuration(String),
}

impl RabbitMqStreamError {
    /// Constrói [`RabbitMqStreamError::Configuration`].
    pub(crate) fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Constrói [`RabbitMqStreamError::Consumer`].
    pub(crate) fn consumer(message: impl Into<String>) -> Self {
        Self::Consumer(message.into())
    }

    /// Constrói [`RabbitMqStreamError::Offset`].
    pub(crate) fn offset(message: impl Into<String>) -> Self {
        Self::Offset(message.into())
    }

    /// Constrói [`RabbitMqStreamError::Subscription`].
    pub(crate) fn subscription(message: impl Into<String>) -> Self {
        Self::Subscription(message.into())
    }

    /// Constrói [`RabbitMqStreamError::Serialization`].
    pub(crate) fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into())
    }

    /// `true` quando o erro representa **perda de conexão**, ou seja,
    /// o consumidor deve reconectar e restaurar o offset (§23).
    pub fn is_connection_lost(&self) -> bool {
        match self {
            Self::Connection(ClientError::Io(_))
            | Self::Connection(ClientError::ConnectionClosed)
            | Self::Connection(ClientError::AlreadyClosed) => true,
            Self::Consumer(message) | Self::Producer(message) | Self::Close(message) => {
                let lower = message.to_lowercase();
                lower.contains("broken pipe")
                    || lower.contains("connection closed")
                    || lower.contains("connection reset")
            }
            _ => false,
        }
    }
}

impl From<StreamCreateError> for RabbitMqStreamError {
    fn from(value: StreamCreateError) -> Self {
        match value {
            // A causa raiz é do `ClientError`: preservá-la é o que
            // permite distinguir "broker reiniciou" de "stream inválido".
            StreamCreateError::Client(source) => Self::Connection(source),
            other => Self::Stream(other.to_string()),
        }
    }
}

impl From<StreamDeleteError> for RabbitMqStreamError {
    fn from(value: StreamDeleteError) -> Self {
        match value {
            StreamDeleteError::Client(source) => Self::Connection(source),
            other => Self::Stream(other.to_string()),
        }
    }
}

impl From<ProducerCreateError> for RabbitMqStreamError {
    fn from(value: ProducerCreateError) -> Self {
        match value {
            ProducerCreateError::Client(source) => Self::Connection(source),
            other => Self::Producer(other.to_string()),
        }
    }
}

impl From<ProducerPublishError> for RabbitMqStreamError {
    fn from(value: ProducerPublishError) -> Self {
        match value {
            ProducerPublishError::Client(source) => Self::Connection(source),
            ProducerPublishError::Closed => Self::ProducerClosed,
            other => Self::Publish(other.to_string()),
        }
    }
}

impl From<ProducerCloseError> for RabbitMqStreamError {
    fn from(value: ProducerCloseError) -> Self {
        match value {
            ProducerCloseError::Client(source) => Self::Connection(source),
            other => Self::Close(other.to_string()),
        }
    }
}

impl From<ConsumerCreateError> for RabbitMqStreamError {
    fn from(value: ConsumerCreateError) -> Self {
        match value {
            ConsumerCreateError::Client(source) => Self::Connection(source),
            other => Self::Consumer(other.to_string()),
        }
    }
}

impl From<ConsumerCloseError> for RabbitMqStreamError {
    fn from(value: ConsumerCloseError) -> Self {
        match value {
            ConsumerCloseError::Client(source) => Self::Connection(source),
            other => Self::Close(other.to_string()),
        }
    }
}

impl From<ConsumerStoreOffsetError> for RabbitMqStreamError {
    fn from(value: ConsumerStoreOffsetError) -> Self {
        match value {
            ConsumerStoreOffsetError::Client(source) => Self::Connection(source),
            ConsumerStoreOffsetError::NameMissing => {
                Self::Offset("consumer_name is required to store the offset".into())
            }
        }
    }
}

/// Conversão para o erro da porta [`gerax_eda::EventBus`].
///
/// O adapter **não** implementa a porta diretamente (serializar exige
/// `E: Serialize`, bound que a porta não expressa — ver
/// [`crate::RabbitMqStreamEventBus`]). Esta conversão permite que a
/// aplicação orquestre `EventStore → EventBus → RabbitMQ Stream`
/// mantendo os erros de infraestrutura dentro do contrato do
/// `gerax-eda` (§22).
impl From<RabbitMqStreamError> for gerax_eda::EventBusError {
    fn from(value: RabbitMqStreamError) -> Self {
        gerax_eda::EventBusError::PublishFailed(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rabbitmq_stream_client::types::ResponseCode;

    #[test]
    fn erro_exibe_categoria_e_contexto() {
        let err = RabbitMqStreamError::Configuration("stream must not be empty".into());
        assert_eq!(
            err.to_string(),
            "configuration error: stream must not be empty"
        );
    }

    #[test]
    fn conversao_para_event_bus_error_cumpre_o_contrato_da_porta() {
        let err = RabbitMqStreamError::PublishNotConfirmed;
        let bus_err: gerax_eda::EventBusError = err.into();

        assert!(matches!(
            bus_err,
            gerax_eda::EventBusError::PublishFailed(_)
        ));
        assert!(bus_err.to_string().contains("not confirmed"));
    }

    #[test]
    fn stream_ja_existente_tem_variante_propria_nao_fatal() {
        let err = RabbitMqStreamError::StreamAlreadyExists("school-events".into());
        assert_eq!(err.to_string(), "stream already exists: school-events");
        assert!(!err.is_connection_lost());
    }

    #[test]
    fn erro_de_socket_e_tratado_como_perda_de_conexao() {
        let io = RabbitMqStreamError::from(ClientError::Io(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "broken pipe",
        )));
        assert!(io.is_connection_lost());

        let wrapped = RabbitMqStreamError::consumer("connection reset by peer");
        assert!(wrapped.is_connection_lost());

        let fatal = RabbitMqStreamError::configuration("bad port");
        assert!(!fatal.is_connection_lost());
    }

    #[test]
    fn fechar_dois_vezes_produz_erro_de_fechamento() {
        // O shutdown idempotente (§19) trata "already closed" como
        // sucesso no consumidor; a conversão permanece fiel ao cliente.
        let err: RabbitMqStreamError = ConsumerCloseError::AlreadyClosed.into();
        assert!(matches!(err, RabbitMqStreamError::Close(_)));
        assert!(err.to_string().contains("already closed"));
    }

    #[test]
    fn erro_de_socket_herdado_do_cliente_vira_perda_de_conexao() {
        // O que acontece quando o broker reinicia: o `ClientError`
        // embrulhado precisa chegar ao `Connection`, senão a
        // reconexão (§23) trata a falha como fatal.
        let refused = ConsumerCreateError::Client(ClientError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Connection refused (os error 111)",
        )));
        let err: RabbitMqStreamError = refused.into();
        assert!(matches!(err, RabbitMqStreamError::Connection(_)));
        assert!(err.is_connection_lost(), "reconexão deveria ser tentada");

        let publish: RabbitMqStreamError = ProducerPublishError::Client(ClientError::Io(
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken pipe"),
        ))
        .into();
        assert!(publish.is_connection_lost());

        let closed: RabbitMqStreamError = ProducerPublishError::Closed.into();
        assert!(matches!(closed, RabbitMqStreamError::ProducerClosed));
    }

    #[test]
    fn erros_do_cliente_sao_convertidos_por_categoria() {
        let create: RabbitMqStreamError = StreamCreateError::Create {
            stream: "s".into(),
            status: ResponseCode::InternalError,
        }
        .into();
        assert!(matches!(create, RabbitMqStreamError::Stream(_)));

        let producer: RabbitMqStreamError =
            ProducerCreateError::StreamDoesNotExist { stream: "s".into() }.into();
        assert!(matches!(producer, RabbitMqStreamError::Producer(_)));

        let publish: RabbitMqStreamError = ProducerPublishError::Timeout.into();
        assert!(matches!(publish, RabbitMqStreamError::Publish(_)));

        let closed_producer: RabbitMqStreamError = ProducerPublishError::Closed.into();
        assert!(matches!(
            closed_producer,
            RabbitMqStreamError::ProducerClosed
        ));

        let offset: RabbitMqStreamError = ConsumerStoreOffsetError::NameMissing.into();
        assert!(matches!(offset, RabbitMqStreamError::Offset(_)));
    }
}
