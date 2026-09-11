use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::similarity::{CosineSimilarity, SimilarityMetric};

use super::{
    VectorDocument, VectorSearchRequest, VectorSearchResult, VectorStore, VectorStoreError,
};

/// Armazenamento vetorial em memória, thread-safe e sem persistência.
///
/// Usa um `RwLock<HashMap>` compartilhado e a métrica de similaridade
/// configurada (padrão: [`CosineSimilarity`]). Documentos com vetores não
/// pontuáveis (ex.: norma zero) são ignorados na busca com um log de debug.
pub struct InMemoryVectorStore {
    documents: Arc<RwLock<HashMap<String, VectorDocument>>>,
    metric: Arc<dyn SimilarityMetric>,
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            metric: Arc::new(CosineSimilarity::new()),
        }
    }
}

impl InMemoryVectorStore {
    /// Cria um armazenamento vazio com similaridade por cosseno.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cria um armazenamento vazio com uma métrica de similaridade customizada.
    pub fn with_metric(metric: Arc<dyn SimilarityMetric>) -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            metric,
        }
    }

    /// Quantidade de documentos armazenados (acesso síncrono de teste).
    pub fn len(&self) -> usize {
        self.documents.blocking_read().len()
    }

    /// Indica se o armazenamento está vazio.
    pub fn is_empty(&self) -> bool {
        self.documents.blocking_read().is_empty()
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&self, document: VectorDocument) -> Result<(), VectorStoreError> {
        self.documents
            .write()
            .await
            .insert(document.id.clone(), document);
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<VectorDocument>, VectorStoreError> {
        Ok(self.documents.read().await.get(id).cloned())
    }

    async fn delete(&self, id: &str) -> Result<(), VectorStoreError> {
        self.documents.write().await.remove(id);
        Ok(())
    }

    async fn search(
        &self,
        request: VectorSearchRequest,
    ) -> Result<Vec<VectorSearchResult>, VectorStoreError> {
        if request.vector.vector.is_empty() {
            return Err(VectorStoreError::InvalidRequest(
                "query vector cannot be empty".into(),
            ));
        }
        if request.limit == 0 {
            return Ok(Vec::new());
        }

        let documents = self.documents.read().await;

        let mut scored: Vec<VectorSearchResult> = Vec::new();

        for document in documents.values() {
            let maybe_score = match &request.filter {
                Some(filter) if !filter.matches(&document.metadata) => continue,
                _ => self
                    .metric
                    .similarity(&request.vector.vector, &document.vector.vector),
            };

            let score = match maybe_score {
                Ok(score) => score,
                Err(err) => {
                    tracing::debug!(
                        document_id = %document.id,
                        error = %err,
                        "skipping document with non-scorable vector"
                    );
                    continue;
                }
            };
            if request.min_score.is_some_and(|min_score| score < min_score) {
                continue;
            }

            scored.push(VectorSearchResult {
                document: document.clone(),
                score,
            });
        }

        scored.sort_by(|a, b| b.score.total_cmp(&a.score));
        scored.truncate(request.limit);
        Ok(scored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::Embedding;
    use crate::vector::{Filter, VectorFilter};
    use serde_json::json;

    fn doc(id: &str, vector: Vec<f32>) -> VectorDocument {
        VectorDocument {
            id: id.to_string(),
            vector: Embedding { vector },
            content: format!("content {id}"),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn upsert_then_get() {
        let store = InMemoryVectorStore::new();
        store.upsert(doc("d1", vec![1.0, 0.0])).await.unwrap();
        let fetched = store.get("d1").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, "d1");
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let store = InMemoryVectorStore::new();
        assert!(store.get("missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn upsert_updates_document() {
        let store = InMemoryVectorStore::new();
        store.upsert(doc("d1", vec![1.0, 0.0])).await.unwrap();
        let mut updated = doc("d1", vec![0.0, 1.0]);
        updated.content = "updated".into();
        store.upsert(updated).await.unwrap();
        let fetched = store.get("d1").await.unwrap().unwrap();
        assert_eq!(fetched.content, "updated");
    }

    #[tokio::test]
    async fn delete_removes_document() {
        let store = InMemoryVectorStore::new();
        store.upsert(doc("d1", vec![1.0, 0.0])).await.unwrap();
        store.delete("d1").await.unwrap();
        assert!(store.get("d1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn search_orders_by_score_desc() {
        let store = InMemoryVectorStore::new();
        store.upsert(doc("close", vec![1.0, 0.9])).await.unwrap();
        store.upsert(doc("far", vec![0.7, 0.5])).await.unwrap();
        store.upsert(doc("exact", vec![1.0, 1.0])).await.unwrap();

        let results = store
            .search(VectorSearchRequest {
                vector: Embedding {
                    vector: vec![1.0, 1.0],
                },
                limit: 10,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].document.id, "exact");
        assert!(results[0].score >= results[1].score);
        assert!(results[1].score >= results[2].score);
    }

    #[tokio::test]
    async fn search_respects_limit() {
        let store = InMemoryVectorStore::new();
        for i in 0..10 {
            let v = vec![i as f32, (10 - i) as f32];
            store.upsert(doc(&format!("d{i}"), v)).await.unwrap();
        }
        let results = store
            .search(VectorSearchRequest {
                vector: Embedding {
                    vector: vec![1.0, 1.0],
                },
                limit: 3,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn search_respects_min_score() {
        let store = InMemoryVectorStore::new();
        store.upsert(doc("exact", vec![1.0, 0.0])).await.unwrap();
        store.upsert(doc("orth", vec![0.0, 1.0])).await.unwrap();

        let results = store
            .search(VectorSearchRequest {
                vector: Embedding {
                    vector: vec![1.0, 0.0],
                },
                limit: 10,
                min_score: Some(0.5),
                filter: None,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "exact");
    }

    #[tokio::test]
    async fn search_applies_metadata_filter() {
        let store = InMemoryVectorStore::new();
        let mut a = doc("rust", vec![1.0, 0.0]);
        a.metadata.insert("language".to_string(), json!("rust"));
        let mut b = doc("python", vec![1.0, 0.0]);
        b.metadata.insert("language".to_string(), json!("python"));
        store.upsert(a).await.unwrap();
        store.upsert(b).await.unwrap();

        let results = store
            .search(VectorSearchRequest {
                vector: Embedding {
                    vector: vec![1.0, 0.0],
                },
                limit: 10,
                min_score: None,
                filter: Some(VectorFilter::and([Filter::eq("language", "rust")])),
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "rust");
    }

    #[tokio::test]
    async fn search_rejects_empty_vector() {
        let store = InMemoryVectorStore::new();
        let result = store
            .search(VectorSearchRequest {
                vector: Embedding { vector: vec![] },
                limit: 10,
                min_score: None,
                filter: None,
            })
            .await;
        assert!(matches!(result, Err(VectorStoreError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn concurrent_upserts_and_search() {
        let store = Arc::new(InMemoryVectorStore::new());
        let mut handles = Vec::new();
        for i in 0..8 {
            let store = store.clone();
            handles.push(tokio::spawn(async move {
                for j in 0..50 {
                    store
                        .upsert(doc(
                            &format!("d{i}-{j}"),
                            vec![i as f32 + 1.0, j as f32 + 1.0],
                        ))
                        .await
                        .unwrap();
                }
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        let results = store
            .search(VectorSearchRequest {
                vector: Embedding {
                    vector: vec![1.0, 1.0],
                },
                limit: 400,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 400);
    }

    #[tokio::test]
    async fn search_skips_zero_norm_documents() {
        let store = InMemoryVectorStore::new();
        store.upsert(doc("zero", vec![0.0, 0.0])).await.unwrap();
        store.upsert(doc("valid", vec![1.0, 0.0])).await.unwrap();

        let results = store
            .search(VectorSearchRequest {
                vector: Embedding {
                    vector: vec![1.0, 0.0],
                },
                limit: 10,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "valid");
    }
}
