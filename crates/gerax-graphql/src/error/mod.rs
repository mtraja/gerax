//! Erros específicos do crate `gerax-graphql`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Erros de execução e validação GraphQL.
#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq)]
pub enum GraphqlError {
    /// Erro de validação de schema, query ou argumentos.
    #[error("validation error: {0}")]
    Validation(String),

    /// Erro durante a execução de um resolver.
    #[error("execution error: {0}")]
    Execution(String),

    /// Erro de autenticação.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Erro de autorização.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Erro de query persistida.
    #[error("persisted query error: {0}")]
    PersistedQuery(String),

    /// Complexidade da query excedeu o limite configurado.
    #[error("complexity exceeded: {0}")]
    ComplexityExceeded(String),

    /// Profundidade da query excedeu o limite configurado.
    #[error("depth exceeded: {0}")]
    DepthExceeded(String),

    /// Erro interno do servidor.
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::GraphqlError;

    #[test]
    fn every_error_variant_is_displayable_and_serializable() {
        let errors = [
            GraphqlError::Validation("validation".to_string()),
            GraphqlError::Execution("execution".to_string()),
            GraphqlError::Unauthorized("unauthorized".to_string()),
            GraphqlError::Forbidden("forbidden".to_string()),
            GraphqlError::PersistedQuery("persisted".to_string()),
            GraphqlError::ComplexityExceeded("complexity".to_string()),
            GraphqlError::DepthExceeded("depth".to_string()),
            GraphqlError::Internal("internal".to_string()),
        ];

        assert!(errors.iter().all(|error| !error.to_string().is_empty()));
        assert!(
            errors
                .iter()
                .all(|error| serde_json::to_string(error).is_ok())
        );
    }
}
