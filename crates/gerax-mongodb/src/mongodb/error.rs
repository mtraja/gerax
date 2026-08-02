use thiserror::Error;

#[derive(Debug, Error)]
pub enum MongoDbError {
    #[error("connection error: {0}")]
    ConnectionError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("entity not found: {0}")]
    NotFoundError(String),
}
