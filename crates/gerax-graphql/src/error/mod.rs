use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphQLError {
    #[error("query error: {0}")]
    QueryError(String),
}
