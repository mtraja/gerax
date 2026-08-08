//! Erros específicos do crate `gerax-rpc`.

use thiserror::Error;

/// Erros de execução RPC.
#[derive(Debug, Error)]
pub enum RpcError {
    /// Erro de transporte ou conexão.
    #[error("transport error: {0}")]
    Transport(String),

    /// Erro de serialização/deserialização.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Recurso não encontrado.
    #[error("not found: {0}")]
    NotFound(String),

    /// Erro interno do servidor.
    #[error("internal error: {0}")]
    Internal(String),

    /// Requisição inválida.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Timeout na requisição.
    #[error("timeout: {0}")]
    Timeout(String),

    /// Erro de autenticação.
    #[error("unauthenticated: {0}")]
    Unauthenticated(String),

    /// Erro de autorização.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Recurso esgotado (ex: rate limit).
    #[error("resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Funcionalidade não implementada.
    #[error("unimplemented: {0}")]
    Unimplemented(String),
}
