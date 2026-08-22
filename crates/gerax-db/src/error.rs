use std::fmt;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("configuration error: {0}")]
    ConfigurationError(String),

    #[error("connection error: {0}")]
    ConnectionError(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("entity not found: {0}")]
    NotFoundError(String),
}

impl DbError {
    pub fn configuration<E: fmt::Display>(err: E) -> Self {
        Self::ConfigurationError(err.to_string())
    }

    pub fn connection<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::ConnectionError(Box::new(err))
    }

    pub fn serialization<E: fmt::Display>(err: E) -> Self {
        Self::SerializationError(err.to_string())
    }

    pub fn not_found<E: fmt::Display>(id: E) -> Self {
        Self::NotFoundError(id.to_string())
    }
}
