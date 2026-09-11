use async_trait::async_trait;

use crate::vector::VectorSearchResult;

use super::{Reranker, RerankerError};

/// Reranker "no-op": devolve os documentos inalterados, preservando a ordem
/// original da recuperação vetorial.
#[derive(Debug, Default)]
pub struct NoopReranker;

impl NoopReranker {
    /// Cria um novo reranker no-op.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Reranker for NoopReranker {
    async fn rerank(
        &self,
        _query: &str,
        documents: Vec<VectorSearchResult>,
    ) -> Result<Vec<VectorSearchResult>, RerankerError> {
        Ok(documents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::Embedding;
    use crate::vector::VectorDocument;

    #[tokio::test]
    async fn noop_returns_documents_unchanged() {
        let reranker = NoopReranker::new();
        let docs = vec![VectorSearchResult {
            document: VectorDocument {
                id: "d1".into(),
                vector: Embedding {
                    vector: vec![1.0, 0.0],
                },
                content: "content".into(),
                metadata: Default::default(),
            },
            score: 0.5,
        }];
        let result = reranker.rerank("query", docs).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].score, 0.5);
    }
}
