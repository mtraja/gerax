//! Erros específicos do crate `gerax-grpc`.

use thiserror::Error;

/// Erros de execução gRPC.
#[derive(Debug, Error)]
pub enum GrpcError {
    /// Erro retornado pelo servidor ou transporte gRPC.
    #[error("rpc error: {0}")]
    RpcError(String),

    /// Erro de serialização/deserialização de mensagens.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// Recurso não encontrado no servidor.
    #[error("not found: {0}")]
    NotFound(String),
}
