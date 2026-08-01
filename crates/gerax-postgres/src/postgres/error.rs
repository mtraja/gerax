use thiserror::Error;

#[derive(Debug, Error)]
pub enum PostgresError {
    #[error("connection error: {0}")]
    ConnectionError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("entity not found: {0}")]
    NotFoundError(String),
}
