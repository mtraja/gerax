//! Tipo de requisição GraphQL.

use serde::{Deserialize, Serialize};

/// Representa uma requisição GraphQL.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GraphqlRequest {
    /// A query GraphQL.
    pub query: String,
    /// O nome da operação.
    pub operation_name: Option<String>,
    /// As variáveis da query.
    pub variables: Option<serde_json::Map<String, serde_json::Value>>,
    /// Extensões da requisição.
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}