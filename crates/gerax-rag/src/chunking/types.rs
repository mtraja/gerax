use thiserror::Error;

use crate::document::{Document, DocumentMetadata};

/// Identificador único de um chunk dentro do armazenamento vetorial.
pub type ChunkId = String;

/// Erros possíveis durante a operação de chunking.
#[derive(Debug, Error)]
pub enum ChunkError {
    /// O documento não possui conteúdo para ser dividido.
    #[error("document has no content to chunk")]
    EmptyDocument,
    /// Falha genérica durante o chunking de um conteúdo específico.
    #[error("failed to chunk content: {0}")]
    Chunking(String),
}

/// Um trecho (parte) de um documento, pronto para ser embutido e indexado.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Identificador único do chunk.
    pub id: ChunkId,
    /// Identificador do documento de origem.
    pub document_id: String,
    /// Posição ordinal do chunk dentro do documento original.
    pub index: usize,
    /// Texto do trecho.
    pub content: String,
    /// Metadados associados ao chunk (ex.: hierarquia de headings em Markdown).
    pub metadata: DocumentMetadata,
}

/// Pedido de chunking: encapsula o documento a ser dividido.
#[derive(Debug, Clone)]
pub struct ChunkRequest {
    /// Documento a ser dividido em trechos.
    pub document: Document,
}

/// Port (abstração) para estratégias de divisão de documentos em chunks.
///
/// Implementações devem ser determinísticas e preservar o conteúdo do documento.
pub trait Chunker: Send + Sync {
    /// Divide o documento do pedido em uma lista de chunks.
    fn chunk(&self, request: ChunkRequest) -> Result<Vec<Chunk>, ChunkError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_is_cloneable() {
        let chunk = Chunk {
            id: "c1".into(),
            document_id: "d1".into(),
            index: 0,
            content: "content".into(),
            metadata: Default::default(),
        };
        let cloned = chunk.clone();
        assert_eq!(cloned.id, "c1");
    }
}
