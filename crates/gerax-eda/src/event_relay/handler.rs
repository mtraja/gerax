//! Processamento de eventos de integração.
//!
//! Um [`IntegrationEventHandler`] recebe o envelope já convertido pelo
//! adapter de origem e executa a ação de negócio/infraestrutura
//! (publicar num broker, indexar, notificar…). O relay decide a ação
//! de confirmação a partir do [`HandlerError`] devolvido.
//!
//! Handlers devem ser **idempotentes**: a entrega é at-least-once e o
//! mesmo evento pode chegar mais de uma vez (por exemplo, se o processo
//! falhar entre a publicação e o ACK na origem).

use async_trait::async_trait;
use thiserror::Error;

use super::envelope::IntegrationEvent;

/// Erro de processamento, classificado para a política de retry.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HandlerError {
    /// Falha transitória: a origem deve reentregar o evento.
    #[error("retryable handler error: {0}")]
    Retryable(String),

    /// Falha definitiva: reentregar não resolve.
    #[error("permanent handler error: {0}")]
    Permanent(String),
}

impl HandlerError {
    /// Cria um erro transitório.
    pub fn retryable(message: impl Into<String>) -> Self {
        Self::Retryable(message.into())
    }

    /// Cria um erro definitivo.
    pub fn permanent(message: impl Into<String>) -> Self {
        Self::Permanent(message.into())
    }

    /// `true` quando o processamento pode ser repetido.
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }

    /// `true` quando a falha é definitiva.
    pub const fn is_permanent(&self) -> bool {
        matches!(self, Self::Permanent(_))
    }
}

/// Consumidor de um evento de integração.
#[async_trait]
pub trait IntegrationEventHandler: Send + Sync {
    /// Processa o evento.
    async fn handle(&self, event: &IntegrationEvent) -> Result<(), HandlerError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classificacao_de_erros_e_estavel() {
        assert!(HandlerError::retryable("offline").is_retryable());
        assert!(!HandlerError::retryable("offline").is_permanent());
        assert!(HandlerError::permanent("invalid").is_permanent());
        assert!(!HandlerError::permanent("invalid").is_retryable());
    }
}
