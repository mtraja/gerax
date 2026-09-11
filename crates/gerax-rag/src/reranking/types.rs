use async_trait::async_trait;
use thiserror::Error;

use crate::vector::VectorSearchResult;

/// Erros possíveis ao reordenar resultados.
#[derive(Debug, Error)]
pub enum RerankerError {
    /// O provider de reranking falhou.
    #[error("reranking failed: {0}")]
    Provider(String),
}

/// Abstração sobre um provider de reranking (cross-encoder, LLM, etc.).
///
/// Recebe os resultados da recuperação vetorial e retorna uma lista reordenada.
/// Implementações devem ser fornecidas em crates separadas de providers.
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Reordena os `documents` dado o texto da `query` original.
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<VectorSearchResult>,
    ) -> Result<Vec<VectorSearchResult>, RerankerError>;
}
