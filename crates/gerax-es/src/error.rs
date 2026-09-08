//! Erros específicos de Event Sourcing.

use thiserror::Error;

use crate::version::Version;

/// Erro de *optimistic concurrency*.
///
/// Ocorre quando um append tenta persistir eventos a partir de uma
/// versão que já não é a atual da stream.
///
/// ```text
/// Version atual = 6
/// Expected      = 5
/// → ConcurrencyError { expected: 5, actual: 6 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("concurrency conflict: expected version {expected}, actual {actual}")]
pub struct ConcurrencyError {
    /// Versão esperada pelo invocador.
    pub expected: Version,
    /// Versão atual da stream.
    pub actual: Version,
}

/// Erros produzidos pelo evento [`crate::EventStore`].
#[derive(Debug, Error)]
pub enum EventStoreError {
    /// Conflito de concorrência no append.
    #[error(transparent)]
    Concurrency(#[from] ConcurrencyError),

    /// Falha inerente ao armazenamento concreto.
    #[error("storage error: {0}")]
    Storage(String),

    /// Evento solicitado não foi encontrado.
    #[error("event not found: {0}")]
    EventNotFound(String),

    /// A sequência de eventos do append não segue a versão esperada.
    #[error("invalid event sequence: {0}")]
    InvalidSequence(String),

    /// Falha de serialização/desserialização de eventos.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Outras falhas não categorizadas.
    #[error("{0}")]
    Other(String),
}

/// Erros de serialização/desserialização de eventos.
#[derive(Debug, Error)]
pub enum SerializationError {
    /// Tipo de evento não registrado/desconhecido.
    #[error("unknown event type `{0}`")]
    UnknownEventType(String),

    /// Payload inválido para o tipo de evento declarado.
    #[error("invalid payload for event `{event_type}`: {source}")]
    InvalidPayload {
        /// Tipo de evento declarado.
        event_type: String,
        /// Erro original de desserialização.
        #[source]
        source: serde_json::Error,
    },

    /// Falha ao serializar o evento.
    #[error("failed to serialize event: {source}")]
    Serialize {
        /// Erro original de serialização.
        #[source]
        source: serde_json::Error,
    },

    /// O payload gerado não declara o campo `event_type`.
    #[error("serialized payload does not declare an `event_type`")]
    MissingEventType,
}

/// Erros produzidos pelo [`crate::AggregateRepository`].
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// Falha do `EventStore` subjacente.
    #[error("event store error: {0}")]
    EventStore(#[from] EventStoreError),

    /// Falha de serialização durante load/save.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Falha na operação de domínio do Aggregate.
    #[error("aggregate error: {0}")]
    Aggregate(String),

    /// Nenhum evento encontrado para o Aggregate solicitado.
    #[error("aggregate not found: {0}")]
    NotFound(String),
}

/// Erros produzidos pelo [`crate::SnapshotStore`].
#[derive(Debug, Error)]
pub enum SnapshotStoreError {
    /// Falha inerente ao armazenamento concreto.
    #[error("storage error: {0}")]
    Storage(String),

    /// Falha de serialização do snapshot.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Outras falhas não categorizadas.
    #[error("{0}")]
    Other(String),
}

/// Erros produzidos pelo [`crate::EventPublisher`].
#[derive(Debug, Error)]
pub enum PublishError {
    /// Falha ao entregar eventos aos consumidores.
    #[error("publish failed: {0}")]
    Publish(String),

    /// Outras falhas não categorizadas.
    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrency_error_carrega_expected_e_actual() {
        let err = ConcurrencyError {
            expected: Version::new(5),
            actual: Version::new(6),
        };

        assert_eq!(err.expected, Version::new(5));
        assert_eq!(err.actual, Version::new(6));
        assert_eq!(
            err.to_string(),
            "concurrency conflict: expected version 5, actual 6"
        );
    }

    #[test]
    fn event_store_error_converte_de_concurrency_error() {
        let concurrency = ConcurrencyError {
            expected: Version::new(3),
            actual: Version::new(4),
        };

        let err: EventStoreError = concurrency.into();
        assert!(matches!(err, EventStoreError::Concurrency(_)));
    }
}
