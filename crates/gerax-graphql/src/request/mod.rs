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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::GraphqlRequest;

    #[test]
    fn request_round_trips_through_json() {
        let request = GraphqlRequest {
            query: "query User($id: ID!) { user(id: $id) { id } }".to_string(),
            operation_name: Some("User".to_string()),
            variables: Some(serde_json::Map::from_iter([("id".to_string(), json!(7))])),
            extensions: Some(serde_json::Map::from_iter([(
                "trace".to_string(),
                json!(true),
            )])),
        };

        let encoded = serde_json::to_string(&request);
        let decoded: Result<GraphqlRequest, _> =
            encoded.and_then(|value| serde_json::from_str(&value));

        assert!(decoded.is_ok());
        if let Ok(decoded) = decoded {
            assert_eq!(decoded, request);
        }
    }
}
