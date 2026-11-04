//! Erros de conexão, configuração e integração com o KurrentDB.
//!
//! O adapter mantém os erros **tipados** por categoria. Os erros do
//! [`gerax_es::EventStoreError`] continuam sendo usados na porta de
//! persistência; erros específicos de subscription vivem em
//! [`crate::subscription::error::SubscriptionError`].

use gerax_es::EventStoreError;
use thiserror::Error;

/// Erros do cliente e da configuração do KurrentDB.
#[derive(Debug, Error)]
pub enum KurrentDbError {
    /// Configuração ausente ou inconsistente.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// Connection string inválida.
    #[error("invalid connection string: {0}")]
    InvalidConnectionString(String),

    /// Falha ao inicializar o cliente gRPC.
    #[error("client initialization error: {0}")]
    ClientInitialization(String),

    /// Falha de conexão ou de comunicação com o cluster.
    #[error("connection error: {0}")]
    Connection(String),

    /// Falha inerente ao armazenamento.
    #[error("storage error: {0}")]
    Storage(String),

    /// Falha de serialização/desserialização.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Falha ao ler o arquivo de configuração.
    #[error("io error: {0}")]
    Io(String),
}

impl From<KurrentDbError> for EventStoreError {
    fn from(value: KurrentDbError) -> Self {
        match value {
            KurrentDbError::Serialization(message) => EventStoreError::Serialization(message),
            KurrentDbError::Storage(message) => EventStoreError::Storage(message),
            other => EventStoreError::Storage(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erro_de_serializacao_preserva_a_variante() {
        let error: EventStoreError = KurrentDbError::Serialization("json".into()).into();
        assert!(matches!(error, EventStoreError::Serialization(_)));
    }

    #[test]
    fn erro_de_conexao_vira_storage_no_event_store() {
        let error: EventStoreError = KurrentDbError::Connection("offline".into()).into();
        assert!(matches!(error, EventStoreError::Storage(_)));
    }

    #[test]
    fn mensagens_sao_legiveis() {
        assert_eq!(
            KurrentDbError::Configuration("empty group".into()).to_string(),
            "configuration error: empty group"
        );
    }
}
