//! Erros específicos do crate `gerax-graphql`.

use thiserror::Error;

/// Erros de execução e validação GraphQL.
#[derive(Debug, Error)]
pub enum GraphQLError {
    /// Erro de sintaxe ou validação de query.
    #[error("query error: {0}")]
    QueryError(String),
}
