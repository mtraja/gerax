//! Tipo de resposta GraphQL.

use crate::error::GraphqlError;
use serde::{Deserialize, Serialize};

/// Representa uma resposta GraphQL.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct GraphqlResponse {
    /// Os dados da resposta.
    pub data: Option<serde_json::Value>,
    /// Os erros da resposta.
    pub errors: Option<Vec<GraphqlError>>,
    /// Extensões da resposta.
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}
