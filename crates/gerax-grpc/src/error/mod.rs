//! Erros específicos do crate `gerax-grpc`.

use thiserror::Error;

use gerax_db::DbError;
use gerax_rpc::error::RpcError;

/// Erros de execução gRPC.
#[derive(Debug, Error)]
pub enum GrpcError {
    /// Erro retornado pelo servidor ou transporte gRPC.
    #[error("rpc error: {0}")]
    RpcError(String),

    /// Requisição inválida (ex: payload malformado).
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Erro de serialização/deserialização de mensagens.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Recurso não encontrado no servidor.
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<DbError> for GrpcError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFoundError(id) => Self::NotFound(id),
            DbError::SerializationError(msg) => Self::SerializationError(msg),
            DbError::ConfigurationError(msg) => Self::InvalidRequest(msg),
            DbError::ConnectionError(_) => Self::RpcError(err.to_string()),
        }
    }
}

impl From<RpcError> for GrpcError {
    fn from(err: RpcError) -> Self {
        Self::RpcError(err.to_string())
    }
}
