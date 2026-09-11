use thiserror::Error;

use super::{Chunk, ChunkError, ChunkRequest, Chunker};

/// Erros de configuração do [`RecursiveChunker`].
#[derive(Debug, Error)]
pub enum RecursiveChunkerError {
    /// `chunk_size` deve ser maior que zero.
    #[error("chunk_size must be greater than zero")]
    InvalidChunkSize,
    /// `overlap` deve ser menor que `chunk_size`.
    #[error("overlap must be less than chunk_size")]
    InvalidOverlap,
    /// Ao menos um separador é obrigatório.
    #[error("at least one separator is required")]
    NoSeparators,
}

/// Divide o documento tentando preservar fronteiras semânticas.
///
/// O texto é dividido recursivamente usando uma lista ordenada de separadores:
/// tenta-se primeiro os maiores (ex.: parágrafos) e, caso os pedaços ainda
/// excedam `chunk_size`, desce para separadores menores (frases, palavras).
#[derive(Debug, Clone)]
pub struct RecursiveChunker {
    /// Separadores em ordem de prioridade (do maior contexto para o menor).
    pub separators: Vec<String>,
    /// Tamanho (em caracteres) máximo de cada chunk.
    pub chunk_size: usize,
    /// Número de caracteres repetidos entre chunks consecutivos.
    pub overlap: usize,
}

impl RecursiveChunker {
    /// Cria um chunker recursivo com os separadores, tamanho e overlap informados.
    ///
    /// # Erros
    ///
    /// Retorna [`RecursiveChunkerError::InvalidChunkSize`] se `chunk_size` for zero,
    /// [`RecursiveChunkerError::InvalidOverlap`] se `overlap >= chunk_size` e
    /// [`RecursiveChunkerError::NoSeparators`] se a lista de separadores for vazia.
    pub fn new(
        separators: Vec<String>,
        chunk_size: usize,
        overlap: usize,
    ) -> Result<Self, RecursiveChunkerError> {
        if chunk_size == 0 {
            return Err(RecursiveChunkerError::InvalidChunkSize);
        }
        if overlap >= chunk_size {
            return Err(RecursiveChunkerError::InvalidOverlap);
        }
        if separators.is_empty() {
            return Err(RecursiveChunkerError::NoSeparators);
        }
        Ok(Self {
            separators,
            chunk_size,
            overlap,
        })
    }
}

impl Default for RecursiveChunker {
    fn default() -> Self {
        Self {
            separators: vec![
                "\n\n".to_string(),
                "\n".to_string(),
                ". ".to_string(),
                " ".to_string(),
            ],
            chunk_size: 512,
            overlap: 32,
        }
    }
}

impl Chunker for RecursiveChunker {
    fn chunk(&self, request: ChunkRequest) -> Result<Vec<Chunk>, ChunkError> {
        let document = request.document;
        if document.content.is_empty() {
            return Err(ChunkError::EmptyDocument);
        }

        let pieces = super::algorithm::split_recursive(
            &document.content,
            &self.separators,
            self.chunk_size,
            self.overlap,
        )?;

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

    fn chunker() -> RecursiveChunker {
        RecursiveChunker::default()
    }

    #[test]
    fn empty_document_fails() {
        let chunker = chunker();
        let doc = Document {
            id: "doc1".into(),
            content: String::new(),
            metadata: Default::default(),
        };
        let result = chunker.chunk(ChunkRequest { document: doc });
        assert!(matches!(result, Err(ChunkError::EmptyDocument)));
    }

    #[test]
    fn short_document_stays_single_chunk() {
        let chunker = chunker();
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document("Frase curta."),
            })
            .unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Frase curta.");
    }

    #[test]
    fn preserves_paragraph_boundaries() {
        let chunker = chunker();
        let text = "Primeiro paragrafo sobre matricula.\n\nSegundo paragrafo sobre calendario.\n\nTerceiro paragrafo sobre biblioteca.";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        assert!(
            chunks
                .iter()
                .any(|c| c.content.starts_with("Primeiro") && c.content.contains("matricula"))
        );
        assert!(chunks.iter().any(|c| c.content.contains("calendario")));
        assert!(!chunks.is_empty());
    }

    #[test]
    fn splits_by_smaller_separators_when_needed() {
        let chunker = RecursiveChunker::new(vec!["\n\n".into(), " ".into()], 16, 2).unwrap();
        let text = "um dois tres quatro cinco seis sete oito";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| !c.content.is_empty()));
    }

    #[test]
    fn unicode_content_not_split_mid_char() {
        let chunker = RecursiveChunker::new(vec![" ".into()], 7, 2).unwrap();
        let text = "Olá João vai à escola com acentuação".repeat(2);
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(&text),
            })
            .unwrap();
        assert!(chunks.iter().all(|c| !c.content.is_empty()));
    }

    #[test]
    fn invalid_configuration() {
        assert!(matches!(
            RecursiveChunker::new(vec!["\n".into()], 0, 1),
            Err(RecursiveChunkerError::InvalidChunkSize)
        ));
        assert!(matches!(
            RecursiveChunker::new(vec!["\n".into()], 10, 10),
            Err(RecursiveChunkerError::InvalidOverlap)
        ));
        assert!(matches!(
            RecursiveChunker::new(vec![], 10, 2),
            Err(RecursiveChunkerError::NoSeparators)
        ));
    }

    #[test]
    fn preserves_order_of_sections() {
        let chunker = chunker();
        let text = "Introducao ao sistema.\n\nSeção de matricula.\n\nRegras de cancelamento.";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        let positions: Vec<usize> = ["Introducao", "matricula", "cancelamento"]
            .iter()
            .map(|needle| {
                chunks
                    .iter()
                    .position(|c| c.content.contains(needle))
                    .unwrap()
            })
            .collect();
        assert!(positions[0] <= positions[1] && positions[1] <= positions[2]);
    }

    #[test]
    fn no_content_loss() {
        let chunker = chunker();
        let text = "palavra a palavra b palavra c palavra d palavra e palavra f palavra g palavra h palavra i palavra j palavra".repeat(2);
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(&text),
            })
            .unwrap();
        assert!(chunks.iter().any(|c| c.content.contains("palavra a")));
        let union: String = chunks.iter().map(|c| c.content.clone()).collect();
        assert!(union.contains(&text[..20]));
    }
}
