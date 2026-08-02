//! Erros específicos do crate `gerax-graphql`.

use thiserror::Error;
use serde::{Deserialize, Serialize};

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