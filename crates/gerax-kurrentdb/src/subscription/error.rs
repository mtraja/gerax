//! Erros específicos de Persistent Subscriptions.
//!
//! Os erros são tipados por categoria (configuração, conexão, grupo,
//! recebimento, ACK/NACK e serialização) para que a camada superior
//! possa decidir entre retry, park ou falha definitiva sem depender de
//! strings ou de tipos internos do cliente KurrentDB.

use thiserror::Error;

use super::ack::SubscriptionAck;

/// Erros do consumidor e do gerenciamento de grupos.
#[derive(Debug, Error)]
pub enum SubscriptionError {
    /// Configuração ausente ou inconsistente.
    #[error("subscription configuration error: {0}")]
    Configuration(String),

    /// Falha de conexão ou de transporte com o cluster.
    #[error("subscription connection error: {0}")]
    Connection(String),

    /// O grupo solicitado não existe.
    #[error("subscription group `{group}` not found on stream `{stream}`")]
    GroupNotFound {
        /// Stream consultada.
        stream: String,
        /// Grupo inexistente.
        group: String,
    },

    /// O grupo já existe.
    #[error("subscription group `{group}` already exists on stream `{stream}`")]
    GroupAlreadyExists {
        /// Stream consultada.
        stream: String,
        /// Grupo já existente.
        group: String,
    },

    /// Falha ao criar, atualizar, listar ou remover grupos.
    #[error("subscription group management error: {0}")]
    GroupManagement(String),

    /// Falha ao receber eventos da subscription.
    #[error("subscription receive error: {0}")]
    Receive(String),

    /// Falha ao confirmar (ACK/NACK) um evento.
    #[error("subscription acknowledgement error: {0}")]
    Acknowledge(String),

    /// Falha de serialização/desserialização.
    #[error("subscription serialization error: {0}")]
    Serialization(String),

    /// A subscription foi encerrada de forma controlada.
    #[error("subscription closed")]
    Closed,
}

impl SubscriptionError {
    /// `true` quando a operação pode ser repetida (falha transitória).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Connection(_) | Self::Receive(_) | Self::Acknowledge(_)
        )
    }

    /// `true` quando repetir a mesma operação não deve resolver.
    pub fn is_permanent(&self) -> bool {
        !self.is_retryable()
    }

    /// Constrói o erro de ACK/NACK preservando a ação solicitada.
    pub(crate) fn acknowledgement(action: SubscriptionAck, message: impl Into<String>) -> Self {
        Self::Acknowledge(format!("{action:?}: {}", message.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falhas_de_transporte_sao_retentaveis() {
        assert!(SubscriptionError::Connection("offline".into()).is_retryable());
        assert!(SubscriptionError::Receive("reset".into()).is_retryable());
        assert!(SubscriptionError::Acknowledge("reset".into()).is_retryable());
    }

    #[test]
    fn falhas_de_configuracao_e_grupo_sao_permanentes() {
        assert!(SubscriptionError::Configuration("x".into()).is_permanent());
        assert!(
            SubscriptionError::GroupNotFound {
                stream: "$all".into(),
                group: "relay".into(),
            }
            .is_permanent()
        );
        assert!(SubscriptionError::Serialization("json".into()).is_permanent());
    }

    #[test]
    fn erro_de_ack_preserva_a_acao() {
        let error = SubscriptionError::acknowledgement(SubscriptionAck::Park, "boom");
        assert!(error.to_string().contains("Park"));
    }

    #[test]
    fn mensagens_incluem_stream_e_grupo() {
        let error = SubscriptionError::GroupAlreadyExists {
            stream: "Aluno-1".into(),
            group: "relay".into(),
        };
        let message = error.to_string();
        assert!(message.contains("Aluno-1"));
        assert!(message.contains("relay"));
    }
}
