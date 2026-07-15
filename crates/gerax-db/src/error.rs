use std::fmt;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("connection error: {0}")]
    ConnectionError(#[source] std::io::Error),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("entity not found: {0}")]
    NotFoundError(String),
}

impl DbError {
    pub fn connection<E: fmt::Display>(err: E) -> Self {
        Self::ConnectionError(std::io::Error::other(err.to_string()))
    }

    pub fn serialization<E: fmt::Display>(err: E) -> Self {
        Self::SerializationError(err.to_string())
    }

    pub fn not_found<E: fmt::Display>(id: E) -> Self {
        Self::NotFoundError(id.to_string())
    }
}
