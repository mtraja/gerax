use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrpcError {
    #[error("rpc error: {0}")]
    RpcError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("not found: {0}")]
    NotFound(String),
}
