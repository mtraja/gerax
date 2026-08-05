//! Cache de respostas GraphQL.

use std::collections::HashMap;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::{GraphqlError, GraphqlRequest, GraphqlResponse};

/// Cache assíncrono, em memória, de respostas GraphQL.
///
/// A chave é derivada de todos os campos da requisição, evitando que queries
/// com variáveis, operação ou extensões diferentes compartilhem uma resposta.
#[derive(Clone, Default)]
pub struct ResponseCache {
    entries: Arc<RwLock<HashMap<String, GraphqlResponse>>>,
}

impl ResponseCache {
    /// Cria um cache de respostas vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Calcula uma chave determinística para uma requisição GraphQL.
    pub fn key_for(request: &GraphqlRequest) -> Result<String, GraphqlError> {
        let serialized = serde_json::to_vec(request).map_err(|error| {
            GraphqlError::Internal(format!("failed to serialize cache key: {error}"))
        })?;
        let digest = Sha256::digest(serialized);
        Ok(hex::encode(digest))
    }

    /// Busca uma resposta armazenada para a requisição.
    pub async fn get(
        &self,
        request: &GraphqlRequest,
    ) -> Result<Option<GraphqlResponse>, GraphqlError> {
        let key = Self::key_for(request)?;
        Ok(self.entries.read().await.get(&key).cloned())
    }

    /// Armazena uma resposta para a requisição.
    pub async fn insert(
        &self,
        request: &GraphqlRequest,
        response: GraphqlResponse,
    ) -> Result<(), GraphqlError> {
        let key = Self::key_for(request)?;
        self.entries.write().await.insert(key, response);
        Ok(())
    }

    /// Remove a resposta associada à requisição.
    pub async fn remove(
        &self,
        request: &GraphqlRequest,
    ) -> Result<Option<GraphqlResponse>, GraphqlError> {
        let key = Self::key_for(request)?;
        Ok(self.entries.write().await.remove(&key))
    }

    /// Remove todas as respostas armazenadas.
    pub async fn clear(&self) {
        self.entries.write().await.clear();
    }

    /// Retorna a quantidade de respostas armazenadas.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Retorna `true` quando o cache está vazio.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ResponseCache;
    use crate::{GraphqlRequest, GraphqlResponse};

    fn request(variable: i32) -> GraphqlRequest {
        GraphqlRequest {
            query: "query User($id: Int!) { user(id: $id) { id } }".to_string(),
            operation_name: Some("User".to_string()),
            variables: Some(serde_json::Map::from_iter([(
                "id".to_string(),
                json!(variable),
            )])),
            extensions: None,
        }
    }

    #[tokio::test]
    async fn stores_and_removes_responses_by_request() {
        let cache = ResponseCache::new();
        let request = request(7);
        let response = GraphqlResponse {
            data: Some(json!({ "user": { "id": 7 } })),
            ..GraphqlResponse::default()
        };

        assert!(cache.is_empty().await);
        assert!(cache.insert(&request, response.clone()).await.is_ok());
        assert_eq!(cache.get(&request).await, Ok(Some(response.clone())));
        assert_eq!(cache.len().await, 1);
        assert_eq!(cache.remove(&request).await, Ok(Some(response)));
        assert!(cache.is_empty().await);
    }

    #[tokio::test]
    async fn cache_key_includes_request_variables() {
        let cache = ResponseCache::new();
        let first = request(7);
        let second = request(8);

        assert!(
            cache
                .insert(
                    &first,
                    GraphqlResponse {
                        data: Some(json!({ "user": { "id": 7 } })),
                        ..GraphqlResponse::default()
                    },
                )
                .await
                .is_ok()
        );

        assert_eq!(cache.get(&second).await, Ok(None));
    }
}
