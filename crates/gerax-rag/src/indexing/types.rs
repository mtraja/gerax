use thiserror::Error;

use crate::chunking::ChunkError;
use crate::embedding::EmbeddingError;
use crate::vector::VectorStoreError;

/// Erros agregados do processo de indexação.
///
/// Consolida falhas de cada estágio (chunking, embedding e armazenamento) em
/// um único tipo de erro com conversão automática via `#[from]`.
#[derive(Debug, Error)]
pub enum IndexError {
    /// Falha durante a divisão do documento em chunks.
    #[error("indexing failed during chunking: {0}")]
    Chunking(#[from] ChunkError),
    /// Falha durante a geração de embeddings.
    #[error("indexing failed during embedding: {0}")]
    Embedding(#[from] EmbeddingError),
    /// Falha durante o armazenamento vetorial.
    #[error("indexing failed during storage: {0}")]
    VectorStore(#[from] VectorStoreError),
    /// Campo obrigatório ausente no builder.
    #[error("builder missing required field: {0}")]
    MissingField(String),
}

/// Resumo da indexação de um documento.
#[derive(Debug, Clone)]
pub struct IndexingResult {
    /// Identificador do documento indexado.
    pub document_id: String,
    /// Quantidade de chunks produzidos pelo chunker.
    pub chunks_created: usize,
    /// Quantidade de chunks persistidos no vector store.
    pub chunks_indexed: usize,
}
