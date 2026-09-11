use std::sync::Arc;

use thiserror::Error;

use crate::embedding::{EmbeddingError, EmbeddingProvider};
use crate::vector::{
    VectorFilter, VectorSearchRequest, VectorSearchResult, VectorStore, VectorStoreError,
};

/// Erros agregados da recuperação.
#[derive(Debug, Error)]
pub enum RetrievalError {
    /// Falha ao gerar o embedding da consulta.
    #[error("embedding failed: {0}")]
    Embedding(#[from] EmbeddingError),
    /// Falha na busca do vector store.
    #[error("vector store failed: {0}")]
    VectorStore(#[from] VectorStoreError),
    /// A requisição de recuperação é inválida.
    #[error("retrieval request invalid: {0}")]
    InvalidRequest(String),
}

/// Requisição de recuperação semântica.
#[derive(Debug, Clone)]
pub struct RetrievalRequest {
    /// Texto da consulta.
    pub query: String,
    /// Quantidade máxima de resultados a retornar.
    pub limit: usize,
    /// Pontuação mínima (similaridade) para manter um resultado.
    pub min_score: Option<f32>,
    /// Filtro de metadados opcional aplicado na busca.
    pub filter: Option<VectorFilter>,
}

/// Resultado de uma recuperação semântica.
#[derive(Debug)]
pub struct RetrievalResult {
    /// Texto da consulta executada.
    pub query: String,
    /// Resultados ordenados por relevância (score decrescente).
    pub results: Vec<VectorSearchResult>,
}

/// Recupera trechos relevantes via embedding da consulta + busca vetorial.
///
/// Encapsula a geração do embedding da consulta e a busca no [`VectorStore`].
pub struct Retriever {
    embedding_provider: Arc<dyn EmbeddingProvider>,
    vector_store: Arc<dyn VectorStore>,
}

impl Retriever {
    /// Cria um retriever com o provider de embeddings e o vector store.
    pub fn new(
        embedding_provider: Arc<dyn EmbeddingProvider>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        Self {
            embedding_provider,
            vector_store,
        }
    }

    /// Recupera resultados relevantes para `request`.
    ///
    /// Rejeita consultas vazias ([`RetrievalError::InvalidRequest`]) e retorna
    /// resultado vazio quando `limit == 0`. Emite um evento de `tracing::debug`
    /// ao concluir.
    pub async fn retrieve(
        &self,
        request: RetrievalRequest,
    ) -> Result<RetrievalResult, RetrievalError> {
        if request.query.is_empty() {
            return Err(RetrievalError::InvalidRequest(
                "query cannot be empty".into(),
            ));
        }
        if request.limit == 0 {
            return Ok(RetrievalResult {
                query: request.query,
                results: Vec::new(),
            });
        }

        let embedding = self.embedding_provider.embed(&request.query).await?;

        let search_request = VectorSearchRequest {
            vector: embedding,
            limit: request.limit,
            min_score: request.min_score,
            filter: request.filter,
        };

        let results = self.vector_store.search(search_request).await?;

        tracing::debug!(
            query = %request.query,
            limit = request.limit,
            results = results.len(),
            "retrieval performed"
        );

        Ok(RetrievalResult {
            query: request.query,
            results,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunking::FixedSizeChunker;
    use crate::chunking::{ChunkRequest, Chunker};
    use crate::embedding::MockEmbeddingProvider;
    use crate::vector::{Filter, InMemoryVectorStore, VectorDocument, VectorFilter};

    async fn setup() -> (Retriever, Arc<dyn EmbeddingProvider>) {
        let provider = Arc::new(MockEmbeddingProvider::default());
        let store = Arc::new(InMemoryVectorStore::new());
        let chunker = FixedSizeChunker::new(64, 8).unwrap();

        let docs = [
            "Para cancelar uma matricula o aluno deve acessar o portal. Preencha o formulario de cancelamento.",
            "As aulas comecam no dia primeiro de marco. Fique atento ao calendario escolar.",
            "Os materiais didaticos devem ser devolvidos ao final do semestre.",
            "O boleto da matricula deve ser pago ate quinze de janeiro.",
        ];

        for text in &docs {
            let chunks = chunker
                .chunk(ChunkRequest {
                    document: crate::document::Document::builder()
                        .id(*text)
                        .content(*text)
                        .build()
                        .unwrap(),
                })
                .unwrap();

            for chunk in chunks {
                let embedding = provider.embed(&chunk.content).await.unwrap();
                store
                    .upsert(VectorDocument {
                        id: chunk.id,
                        vector: embedding,
                        content: chunk.content,
                        metadata: chunk.metadata,
                    })
                    .await
                    .unwrap();
            }
        }

        (Retriever::new(provider.clone(), store), provider)
    }

    #[tokio::test]
    async fn retrieve_returns_relevant_chunks() {
        let (retriever, _) = setup().await;
        let result = retriever
            .retrieve(RetrievalRequest {
                query: "como cancelar matricula".into(),
                limit: 5,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();

        assert_eq!(result.query, "como cancelar matricula");
        assert!(!result.results.is_empty());
        let top_result = &result.results[0].document.content;
        assert!(
            top_result.contains("cancelar") || top_result.contains("matricula"),
            "expected relevant chunk, got: {top_result}"
        );
    }

    #[tokio::test]
    async fn retrieve_empty_query_fails() {
        let (retriever, _) = setup().await;
        let result = retriever
            .retrieve(RetrievalRequest {
                query: String::new(),
                limit: 5,
                min_score: None,
                filter: None,
            })
            .await;
        assert!(matches!(result, Err(RetrievalError::InvalidRequest(_))));
    }

    #[tokio::test]
    async fn retrieve_limit_zero_returns_empty() {
        let (retriever, _) = setup().await;
        let result = retriever
            .retrieve(RetrievalRequest {
                query: "matricula".into(),
                limit: 0,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn retrieve_respects_min_score() {
        let (retriever, _) = setup().await;
        let result = retriever
            .retrieve(RetrievalRequest {
                query: "calendario escolar aulas".into(),
                limit: 10,
                min_score: Some(0.3),
                filter: None,
            })
            .await
            .unwrap();
        for r in &result.results {
            assert!(r.score >= 0.3);
        }
    }

    #[tokio::test]
    async fn retrieve_applies_metadata_filter() {
        let provider = Arc::new(MockEmbeddingProvider::default());
        let store = Arc::new(InMemoryVectorStore::new());
        let chunker = FixedSizeChunker::new(64, 8).unwrap();

        let indexed = [
            ("manual-matricula", "matricula", "school"),
            ("manual-biblioteca", "biblioteca", "school"),
            ("projeto-notas", "anotacoes da reuniao", "project-a"),
        ];

        for (doc_id, content, repo) in indexed {
            let chunks = chunker
                .chunk(ChunkRequest {
                    document: crate::document::Document::builder()
                        .id(doc_id)
                        .content(content)
                        .metadata("repository", repo)
                        .build()
                        .unwrap(),
                })
                .unwrap();
            for chunk in chunks {
                let embedding = provider.embed(&chunk.content).await.unwrap();
                store
                    .upsert(VectorDocument {
                        id: chunk.id,
                        vector: embedding,
                        content: chunk.content,
                        metadata: chunk.metadata,
                    })
                    .await
                    .unwrap();
            }
        }

        let retriever = Retriever::new(provider, store);
        let result = retriever
            .retrieve(RetrievalRequest {
                query: "matricula escolar".into(),
                limit: 10,
                min_score: None,
                filter: Some(VectorFilter::and([Filter::eq("repository", "school")])),
            })
            .await
            .unwrap();

        assert!(!result.results.is_empty());
        assert!(
            result
                .results
                .iter()
                .all(|r| r.document.id.starts_with("manual-"))
        );
    }
}
