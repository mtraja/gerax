//! Erros de execução CQRS.

use thiserror::Error;

/// Erros produzidos pela infraestrutura CQRS.
#[derive(Debug, Error)]
pub enum CqrsError {
    /// Nenhum handler foi registrado para a mensagem.
    #[error("handler not found for `{message}`")]
    HandlerNotFound {
        /// Nome da mensagem sem handler.
        message: &'static str,
    },

    /// Já existe um handler registrado para a mesma mensagem.
    #[error("handler already registered")]
    HandlerAlreadyRegistered,

    /// O tipo de mensagem recebido não corresponde ao esperado.
    #[error("invalid message type")]
    InvalidMessageType,

    /// O tipo de saída produzido não corresponde ao esperado.
    #[error("invalid output type")]
    InvalidOutputType,

    /// Falha durante a execução do handler.
    #[error("handler execution failed: {0}")]
    Execution(String),
}
