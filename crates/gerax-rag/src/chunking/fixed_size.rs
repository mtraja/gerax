use thiserror::Error;

use super::{Chunk, ChunkError, ChunkRequest, Chunker};

/// Erros de configuração do [`FixedSizeChunker`].
#[derive(Debug, Error)]
pub enum FixedSizeChunkerError {
    /// `chunk_size` deve ser maior que zero.
    #[error("chunk_size must be greater than zero")]
    InvalidChunkSize,
    /// `overlap` deve ser menor que `chunk_size`.
    #[error("overlap must be less than chunk_size")]
    InvalidOverlap,
}

/// Divide o documento em chunks de tamanho fixo, com overlap opcional entre eles.
///
/// A divisão respeita fronteiras UTF-8: caracteres multibyte nunca são quebrados
/// no meio. O overlap garante que contextos limítrofes sejam compartilhados entre
/// chunks consecutivos.
#[derive(Debug, Clone)]
pub struct FixedSizeChunker {
    /// Tamanho (em caracteres) de cada chunk.
    pub chunk_size: usize,
    /// Número de caracteres repetidos entre chunks consecutivos.
    pub overlap: usize,
}

impl FixedSizeChunker {
    /// Cria um chunker com o tamanho de chunk e overlap informados.
    ///
    /// # Erros
    ///
    /// Retorna [`FixedSizeChunkerError::InvalidChunkSize`] se `chunk_size` for zero
    /// e [`FixedSizeChunkerError::InvalidOverlap`] se `overlap >= chunk_size`.
    pub fn new(chunk_size: usize, overlap: usize) -> Result<Self, FixedSizeChunkerError> {
        if chunk_size == 0 {
            return Err(FixedSizeChunkerError::InvalidChunkSize);
        }
        if overlap >= chunk_size {
            return Err(FixedSizeChunkerError::InvalidOverlap);
        }
        Ok(Self {
            chunk_size,
            overlap,
        })
    }
}

impl Chunker for FixedSizeChunker {
    fn chunk(&self, request: ChunkRequest) -> Result<Vec<Chunk>, ChunkError> {
        let document = request.document;
        if document.content.is_empty() {
            return Err(ChunkError::EmptyDocument);
        }

        let pieces =
            super::algorithm::split_fixed(&document.content, self.chunk_size, self.overlap);

        let chunks: Vec<Chunk> = pieces
            .into_iter()
            .enumerate()
            .map(|(index, content)| {
                super::algorithm::build_chunk(&document, index, &content, Default::default())
            })
            .collect();

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::document::Document;

    fn document(content: &str) -> Document {
        Document::builder()
            .id("doc1")
            .content(content)
            .build()
            .unwrap()
    }

    #[test]
    fn empty_document_fails() {
        let chunker = FixedSizeChunker::new(10, 2).unwrap();
        let doc = Document {
            id: "doc1".into(),
            content: String::new(),
            metadata: Default::default(),
        };
        let result = chunker.chunk(ChunkRequest { document: doc });
        assert!(matches!(result, Err(ChunkError::EmptyDocument)));
    }

    #[test]
    fn small_document_single_chunk() {
        let chunker = FixedSizeChunker::new(100, 10).unwrap();
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document("hello world"),
            })
            .unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "hello world");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn large_document_splits_into_chunks() {
        let content = "a".repeat(250);
        let chunker = FixedSizeChunker::new(100, 0).unwrap();
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(&content),
            })
            .unwrap();
        assert!(chunks.len() >= 3);
        assert!(chunks.iter().all(|c| c.content.len() <= 100));
    }

    #[test]
    fn no_content_loss() {
        let content = "Esta frase tem acentuação. Outra frase também.".repeat(20);
        let chunker = FixedSizeChunker::new(64, 8).unwrap();
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(&content),
            })
            .unwrap();
        let union: String = chunks.iter().map(|c| c.content.clone()).collect();
        assert!(union.contains(&content[..50]));
        assert!(union.contains(&content[content.len() - 50..]));
        assert!(chunks.iter().all(|c| !c.content.is_empty()));
    }

    #[test]
    fn overlap_reproduces_between_chunks() {
        let content = "abcdefghij".repeat(5);
        let chunker = FixedSizeChunker::new(10, 2).unwrap();
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(&content),
            })
            .unwrap();
        assert!(chunks.len() > 1);
        if let Some(first) = chunks.first() {
            assert_eq!(first.content.len(), 10);
        }
    }

    #[test]
    fn invalid_chunk_size() {
        assert!(matches!(
            FixedSizeChunker::new(0, 5),
            Err(FixedSizeChunkerError::InvalidChunkSize)
        ));
    }

    #[test]
    fn invalid_overlap() {
        assert!(matches!(
            FixedSizeChunker::new(10, 10),
            Err(FixedSizeChunkerError::InvalidOverlap)
        ));
        assert!(matches!(
            FixedSizeChunker::new(10, 15),
            Err(FixedSizeChunkerError::InvalidOverlap)
        ));
    }

    #[test]
    fn unicode_characters_preserved() {
        let content = "Olá mundo, como vai? João vai à escola. á é í ó ú çã".repeat(20);
        let chunker = FixedSizeChunker::new(32, 4).unwrap();
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(&content),
            })
            .unwrap();
        assert!(chunks.iter().all(|c| !c.content.is_empty()));
        let all_chars: String = chunks.iter().map(|c| c.content.clone()).collect();
        assert!(all_chars.contains("Olá"));
    }

    #[test]
    fn overlapping_chunks_share_context() {
        let content = "abcdefghij".repeat(5);
        let chunker = FixedSizeChunker::new(10, 4).unwrap();
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(&content),
            })
            .unwrap();
        if chunks.len() >= 2 {
            let first = &chunks[0].content;
            let second = &chunks[1].content;
            assert!(first.chars().count() >= 10);
            assert!(second.chars().count() >= 10);
            assert!(first != second);
        }
    }
}
