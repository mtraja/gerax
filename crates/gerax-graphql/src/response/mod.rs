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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::GraphqlResponse;
    use crate::GraphqlError;

    #[test]
    fn response_round_trips_data_errors_and_extensions() {
        let response = GraphqlResponse {
            data: Some(json!({ "viewer": { "id": "user-42" } })),
            errors: Some(vec![GraphqlError::Execution("resolver failed".to_string())]),
            extensions: Some(serde_json::Map::from_iter([(
                "cached".to_string(),
                json!(false),
            )])),
        };

        let encoded = serde_json::to_string(&response);
        let decoded: Result<GraphqlResponse, _> =
            encoded.and_then(|value| serde_json::from_str(&value));

        assert!(decoded.is_ok());
        if let Ok(decoded) = decoded {
            assert_eq!(decoded, response);
        }
    }
}
