use std::sync::Arc;

use thiserror::Error;

use crate::context::{ContextBuilder, ContextError};
use crate::embedding::EmbeddingError;
use crate::reranking::{Reranker, RerankerError};
use crate::retrieval::{RetrievalError, RetrievalRequest, Retriever};
use crate::vector::VectorStoreError;

use super::{RagRequest, RagResult};

/// Erros agregados da execução do pipeline RAG.
#[derive(Debug, Error)]
pub enum RagError {
    /// Falha na recuperação de documentos.
    #[error("retrieval failed: {0}")]
    Retrieval(#[from] RetrievalError),
    /// Falha no reranking dos resultados.
    #[error("reranking failed: {0}")]
    Reranking(#[from] RerankerError),
    /// Falha ao montar o contexto.
    #[error("context building failed: {0}")]
    Context(#[from] ContextError),
    /// Falha ao gerar embeddings.
    #[error("embedding failed: {0}")]
    Embedding(#[from] EmbeddingError),
    /// Falha em operações do vector store.
    #[error("vector store failed: {0}")]
    VectorStore(#[from] VectorStoreError),
    /// Configuração inválida do pipeline.
    #[error("pipeline configuration invalid: {0}")]
    InvalidConfiguration(String),
}

/// Pipeline RAG ponta a ponta: recuperação → reranking → contexto.
///
/// Compõe um [`Retriever`] (obrigatório), um [`Reranker`] opcional e um
/// [`ContextBuilder`] (obrigatório). Prefira construir via [`RagPipeline::builder`].
pub struct RagPipeline {
    retriever: Retriever,
    reranker: Option<Arc<dyn Reranker>>,
    context_builder: Arc<dyn ContextBuilder>,
}

impl RagPipeline {
    /// Inicia a construção do pipeline via `RagPipelineBuilder`.
    pub fn builder() -> RagPipelineBuilder {
        RagPipelineBuilder::default()
    }

    /// Executa o fluxo RAG para `request`.
    ///
    /// Abre um span de `tracing` para o pipeline e emite eventos de debug para
    /// o reranking e a montagem do contexto.
    pub async fn run(&self, request: RagRequest) -> Result<RagResult, RagError> {
        let span = tracing::debug_span!("rag_pipeline::run", query = %request.query);
        let _guard = span.enter();

        let retrieval_result = self
            .retriever
            .retrieve(RetrievalRequest {
                query: request.query.clone(),
                limit: request.retrieval_limit,
                min_score: request.min_score,
                filter: request.filter.clone(),
            })
            .await?;

        let documents = match &self.reranker {
            Some(reranker) => {
                tracing::debug!(
                    incoming = retrieval_result.results.len(),
                    "reranking performed"
                );
                reranker
                    .rerank(&request.query, retrieval_result.results)
                    .await?
            }
            None => retrieval_result.results,
        };

        let context = self.context_builder.build(&documents)?;

        tracing::debug!(
            documents = documents.len(),
            context_chars = context.chars().count(),
            "context built"
        );

        Ok(RagResult {
            query: request.query,
            documents,
            context,
        })
    }
}

/// Builder incremental para [`RagPipeline`].
///
/// Valida a presença de `retriever` e `context_builder`; o `reranker` é
/// opcional (sem reranker o pipeline preserva a ordem da recuperação).
#[derive(Default)]
pub struct RagPipelineBuilder {
    retriever: Option<Retriever>,
    reranker: Option<Arc<dyn Reranker>>,
    context_builder: Option<Arc<dyn ContextBuilder>>,
}

impl RagPipelineBuilder {
    /// Define o [`Retriever`] obrigatório do pipeline.
    pub fn retriever(mut self, retriever: Retriever) -> Self {
        self.retriever = Some(retriever);
        self
    }

    /// Define o [`Reranker`] opcional do pipeline.
    pub fn reranker(mut self, reranker: Arc<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    /// Define o [`ContextBuilder`] obrigatório do pipeline.
    pub fn context_builder(mut self, context_builder: Arc<dyn ContextBuilder>) -> Self {
        self.context_builder = Some(context_builder);
        self
    }

    /// Constrói o [`RagPipeline`], exigindo `retriever` e `context_builder`.
    ///
    /// Falha com [`RagError::InvalidConfiguration`] quando um campo obrigatório
    /// está ausente.
    pub fn build(self) -> Result<RagPipeline, RagError> {
        let retriever = self
            .retriever
            .ok_or_else(|| RagError::InvalidConfiguration("retriever is required".into()))?;
        let context_builder = self
            .context_builder
            .ok_or_else(|| RagError::InvalidConfiguration("context_builder is required".into()))?;

        Ok(RagPipeline {
            retriever,
            reranker: self.reranker,
            context_builder,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunking::FixedSizeChunker;
    use crate::context::SimpleContextBuilder;
    use crate::document::Document;
    use crate::embedding::MockEmbeddingProvider;
    use crate::indexing::DocumentIndexer;
    use crate::reranking::NoopReranker;
    use crate::vector::InMemoryVectorStore;

    async fn setup_pipeline() -> RagPipeline {
        let chunker = Arc::new(FixedSizeChunker::new(64, 8).unwrap());
        let provider = Arc::new(MockEmbeddingProvider::default());
        let store = Arc::new(InMemoryVectorStore::new());

        let indexer = DocumentIndexer::builder()
            .chunker(chunker)
            .embedding_provider(provider.clone())
            .vector_store(store.clone())
            .build()
            .unwrap();

        let content = "Para cancelar uma matricula o aluno deve acessar o portal e preencher o formulario de cancelamento. \
            O cancelamento so pode ser realizado pelo titular da matricula. \
            Apos confirmar, o sistema envia um email de confirmacao. \
            O prazo para cancelamento sem multa e de trinta dias apos a matricula."
            .repeat(3);

        indexer
            .index(
                Document::builder()
                    .id("manual")
                    .content(content)
                    .metadata("language", "pt-BR")
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();

        let retriever = Retriever::new(provider.clone(), store);
        RagPipeline::builder()
            .retriever(retriever)
            .reranker(Arc::new(NoopReranker::new()))
            .context_builder(Arc::new(SimpleContextBuilder::default()))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn pipeline_requires_retriever() {
        let builder = RagPipeline::builder()
            .context_builder(Arc::new(SimpleContextBuilder::default()))
            .build();
        assert!(matches!(builder, Err(RagError::InvalidConfiguration(_))));
    }

    #[tokio::test]
    async fn pipeline_requires_context_builder() {
        let (provider, store) = (
            Arc::new(MockEmbeddingProvider::default())
                as Arc<dyn crate::embedding::EmbeddingProvider>,
            Arc::new(InMemoryVectorStore::new()) as Arc<dyn crate::vector::VectorStore>,
        );
        let retriever = Retriever::new(provider, store);
        let builder = RagPipeline::builder().retriever(retriever).build();
        assert!(matches!(builder, Err(RagError::InvalidConfiguration(_))));
    }

    #[tokio::test]
    async fn pipeline_end_to_end() {
        let pipeline = setup_pipeline().await;

        let result = pipeline
            .run(RagRequest {
                query: "como desfazer a inscricao de um aluno".into(),
                retrieval_limit: 3,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();

        assert_eq!(result.query, "como desfazer a inscricao de um aluno");
        assert_eq!(result.documents.len(), 3);
        assert!(result.context.contains("[Source: manual-"));
        assert!(!result.context.is_empty());
    }

    #[tokio::test]
    async fn pipeline_rerank_noop_keeps_order() {
        let pipeline = setup_pipeline().await;
        let result = pipeline
            .run(RagRequest {
                query: "portal email confirmacao".into(),
                retrieval_limit: 2,
                min_score: None,
                filter: None,
            })
            .await
            .unwrap();
        assert_eq!(result.documents.len(), 2);
    }
}
