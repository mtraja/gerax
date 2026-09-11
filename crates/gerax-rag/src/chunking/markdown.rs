use thiserror::Error;

use crate::document::{Document, DocumentMetadata};

use super::{Chunk, ChunkError, ChunkRequest, Chunker};

/// Erros de configuração do [`MarkdownChunker`].
#[derive(Debug, Error)]
pub enum MarkdownChunkerError {
    /// `max_chunk_size` deve ser maior que zero.
    #[error("max_chunk_size must be greater than zero")]
    InvalidMaxChunkSize,
}

/// Divide documentos Markdown respeitando a hierarquia de headings (`#` a `######`).
///
/// Cada seção vira um chunk que inclui o cabeçalho completo da hierarquia atual
/// (`# Alunos` + `## Matrícula`), preservando o contexto. Seções que excedem
/// `max_chunk_size` são subdivididas por tamanho fixo, mantendo o cabeçalho no
/// início de cada pedaço gerado.
///
/// Os chunks recebem metadados `heading` (título da seção mais recente) e
/// `heading_path` (caminho completo, ex.: `"Alunos > Matrícula"`).
#[derive(Debug, Clone)]
pub struct MarkdownChunker {
    /// Tamanho máximo (em caracteres) para cada chunk gerado.
    pub max_chunk_size: usize,
}

impl MarkdownChunker {
    /// Cria um chunker Markdown com o tamanho máximo informado.
    ///
    /// # Erros
    ///
    /// Retorna [`MarkdownChunkerError::InvalidMaxChunkSize`] se `max_chunk_size` for zero.
    pub fn new(max_chunk_size: usize) -> Result<Self, MarkdownChunkerError> {
        if max_chunk_size == 0 {
            return Err(MarkdownChunkerError::InvalidMaxChunkSize);
        }
        Ok(Self { max_chunk_size })
    }
}

impl Default for MarkdownChunker {
    fn default() -> Self {
        Self {
            max_chunk_size: 512,
        }
    }
}

/// Um heading (cabeçalho) do documento Markdown.
struct Heading {
    /// Nível do heading (1 a 6, conforme o número de `#`).
    level: usize,
    /// Texto do título, sem os marcadores `#`.
    title: String,
    /// Linha original do heading (com os marcadores).
    raw: String,
}

impl Chunker for MarkdownChunker {
    fn chunk(&self, request: ChunkRequest) -> Result<Vec<Chunk>, ChunkError> {
        let document = request.document;
        if document.content.is_empty() {
            return Err(ChunkError::EmptyDocument);
        }

        let mut chunks = Vec::new();
        let mut headings: Vec<Heading> = Vec::new();
        let mut body_lines: Vec<String> = Vec::new();
        let mut section_index = 0usize;
        let max_chunk_size = self.max_chunk_size;

        let flush = |headings: &[Heading],
                     body: &mut Vec<String>,
                     document: &Document,
                     index: &mut usize,
                     chunks: &mut Vec<Chunk>| {
            if headings.is_empty() && body.is_empty() {
                return;
            }

            let mut section_text = headings
                .iter()
                .map(|h| h.raw.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            if !section_text.is_empty() && !body.is_empty() {
                section_text.push('\n');
                section_text.push('\n');
            }
            section_text.push_str(&body.join("\n"));

            if section_text.trim().is_empty() {
                body.clear();
                return;
            }

            let metadata = heading_metadata(headings);

            let section_chars = section_text.chars().count();
            if section_chars <= max_chunk_size {
                push_section(document, *index, &section_text, metadata, chunks);
                *index += 1;
                body.clear();
                return;
            }

            let headings_block = headings
                .iter()
                .map(|h| h.raw.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let body_text = body.join("\n");
            let pieces = super::algorithm::split_fixed(&body_text, max_chunk_size, 16);
            for piece in pieces {
                let mut combined = String::new();
                if !headings_block.is_empty() {
                    combined.push_str(&headings_block);
                    combined.push_str("\n\n");
                }
                combined.push_str(&piece);
                push_section(document, *index, combined.trim(), metadata.clone(), chunks);
                *index += 1;
            }
            body.clear();
        };

        for line in document.content.lines() {
            if let Some(parsed) = parse_heading(line) {
                flush(
                    &headings,
                    &mut body_lines,
                    &document,
                    &mut section_index,
                    &mut chunks,
                );
                while headings.last().is_some_and(|h| h.level >= parsed.level) {
                    headings.pop();
                }
                headings.push(parsed);
            } else {
                body_lines.push(line.to_string());
            }
        }

        flush(
            &headings,
            &mut body_lines,
            &document,
            &mut section_index,
            &mut chunks,
        );

        if chunks.is_empty() {
            push_section(
                &document,
                0,
                document.content.trim(),
                DocumentMetadata::new(),
                &mut chunks,
            );
        }

        Ok(chunks)
    }
}

/// Tenta interpretar uma linha como heading Markdown (`#` a `######`).
///
/// Retorna `None` para linhas sem heading ou com formato inválido
/// (ex.: `#sem espaco`).
fn parse_heading(line: &str) -> Option<Heading> {
    let trimmed = line.trim_end();
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let after = &trimmed[level..];
    if !after.starts_with(' ') && !after.is_empty() {
        return None;
    }
    let title = after.trim();
    if title.is_empty() {
        return None;
    }
    Some(Heading {
        level,
        title: title.to_string(),
        raw: trimmed.to_string(),
    })
}

/// Gera os metadados `heading` e `heading_path` a partir da hierarquia atual.
fn heading_metadata(headings: &[Heading]) -> DocumentMetadata {
    let mut metadata = DocumentMetadata::new();
    if let Some(last) = headings.last() {
        metadata.insert(
            "heading".to_string(),
            serde_json::Value::String(last.title.clone()),
        );
    }
    if !headings.is_empty() {
        let path = headings
            .iter()
            .map(|h| h.title.as_str())
            .collect::<Vec<_>>()
            .join(" > ");
        metadata.insert("heading_path".to_string(), serde_json::Value::String(path));
    }
    metadata
}

/// Cria um chunk a partir de uma seção e o adiciona à lista de chunks.
fn push_section(
    document: &Document,
    index: usize,
    content: &str,
    extra: DocumentMetadata,
    chunks: &mut Vec<Chunk>,
) {
    chunks.push(super::algorithm::build_chunk(
        document, index, content, extra,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(content: &str) -> Document {
        Document::builder()
            .id("doc1")
            .content(content)
            .build()
            .unwrap()
    }

    #[test]
    fn empty_document_fails() {
        let chunker = MarkdownChunker::default();
        let doc = Document {
            id: "doc1".into(),
            content: String::new(),
            metadata: Default::default(),
        };
        let result = chunker.chunk(ChunkRequest { document: doc });
        assert!(matches!(result, Err(ChunkError::EmptyDocument)));
    }

    #[test]
    fn splits_by_top_level_heading() {
        let chunker = MarkdownChunker::default();
        let text =
            "# Alunos\n\nConteudo sobre alunos.\n\n# Professores\n\nConteudo sobre professores.";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        assert_eq!(chunks.len(), 2);
        let first = chunks
            .iter()
            .find(|c| c.content.contains("Alunos"))
            .unwrap();
        assert!(!first.content.contains("Professores"));
        assert_eq!(first.metadata["heading"].as_str().unwrap(), "Alunos");
    }

    #[test]
    fn preserves_hierarchy_with_heading_path() {
        let chunker = MarkdownChunker::default();
        let text = "# Alunos\n\n## Matricula\n\nPreencha o formulario de matricula.\n\n## Calendario\n\nAulas comecam em marco.";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();

        let matricula = chunks
            .iter()
            .find(|c| c.content.contains("formulario de matricula"))
            .unwrap();
        assert_eq!(
            matricula.metadata["heading_path"].as_str().unwrap(),
            "Alunos > Matricula"
        );
        assert!(matricula.content.contains("# Alunos"));
        assert!(matricula.content.contains("## Matricula"));
    }

    #[test]
    fn content_after_nested_heading() {
        let chunker = MarkdownChunker::default();
        let text = "# Alunos\n\n## Matricula\n\nConteudo...";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        let chunk = chunks
            .iter()
            .find(|c| c.content.contains("Conteudo..."))
            .unwrap();
        assert!(chunk.content.contains("# Alunos"));
        assert!(chunk.content.contains("## Matricula"));
    }

    #[test]
    fn heading_levels_1_to_3() {
        let chunker = MarkdownChunker::default();
        let text = "## Subsecao\n\nConteudo 1.\n\n### Detalhe\n\nConteudo 2.";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        let sub = chunks
            .iter()
            .find(|c| c.content.contains("Conteudo 1"))
            .unwrap();
        assert_eq!(sub.metadata["heading"].as_str().unwrap(), "Subsecao");
        let detail = chunks
            .iter()
            .find(|c| c.content.contains("Conteudo 2"))
            .unwrap();
        assert_eq!(
            detail.metadata["heading_path"].as_str().unwrap(),
            "Subsecao > Detalhe"
        );
    }

    #[test]
    fn section_exceeding_size_is_further_split() {
        let chunker = MarkdownChunker::new(24).unwrap();
        let text = "# Grande\n\nparagrafo com muitas palavras para exceder o limite do chunk";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.content.contains("# Grande"));
        }
    }

    #[test]
    fn plain_text_without_headings_is_one_chunk() {
        let chunker = MarkdownChunker::default();
        let text = "Texto simples sem nenhum heading no documento.";
        let chunks = chunker
            .chunk(ChunkRequest {
                document: document(text),
            })
            .unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(!chunks[0].metadata.contains_key("heading"));
    }
}
