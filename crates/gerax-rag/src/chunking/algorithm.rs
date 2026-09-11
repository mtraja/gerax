use crate::document::Document;
use crate::document::DocumentMetadata;

use super::{Chunk, ChunkError};

/// Constrói um [`Chunk`] a partir do documento de origem, herdando os metadados
/// e adicionando `chunk_index` e eventuais metadados extras.
pub(crate) fn build_chunk(
    document: &Document,
    index: usize,
    content: &str,
    extra_metadata: DocumentMetadata,
) -> Chunk {
    let mut metadata = document.metadata.clone();
    metadata.insert(
        "chunk_index".to_string(),
        serde_json::Value::from(index as u64),
    );
    for (key, value) in extra_metadata {
        metadata.insert(key, value);
    }
    Chunk {
        id: format!("{}-{}", document.id, index),
        document_id: document.id.clone(),
        index,
        content: content.to_owned(),
        metadata,
    }
}

/// Avança até a próxima fronteira UTF-8 válida a partir de um byte.
pub(crate) fn next_char_boundary(content: &str, from: usize) -> usize {
    let mut idx = from;
    while idx < content.len() && !content.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

/// Recua até a fronteira UTF-8 válida anterior (inclusive) a partir de um byte.
pub(crate) fn char_boundary_before(content: &str, from: usize) -> usize {
    let mut idx = from;
    while idx > 0 && !content.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Divide o texto em pedaços de tamanho fixo, respeitando fronteiras UTF-8.
///
/// Cada pedaço subsequente começa `step = chunk_size - overlap` caracteres
/// depois do início do anterior, o que garante o compartilhamento de contexto.
pub(crate) fn split_fixed(content: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let step = chunk_size.saturating_sub(overlap).max(1);
    let mut pieces = Vec::new();

    if content.len() <= chunk_size {
        pieces.push(content.to_string());
        return pieces;
    }

    let mut start = 0;
    while start < content.len() {
        let mut end = (start + chunk_size).min(content.len());
        if end < content.len() && !content.is_char_boundary(end) {
            end = next_char_boundary(content, end);
        }
        pieces.push(content[start..end].to_string());
        if end == content.len() {
            break;
        }
        start += step;
        if start > content.len() {
            break;
        }
        if !content.is_char_boundary(start) {
            start = next_char_boundary(content, start);
        }
    }

    pieces
}

/// Divisão recursiva base: segmenta o texto usando o maior separador presente e,
/// quando um pedaço ainda excede `chunk_size`, subdivide com o próximo separador.
fn split_recursive_base(text: &str, separators: &[String], chunk_size: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let Some(separator_index) = separators
        .iter()
        .position(|sep| text.contains(sep.as_str()))
    else {
        let mut end = chunk_size.min(text.len());
        if !text.is_char_boundary(end) {
            end = char_boundary_before(text, end);
        }
        let part = &text[..end];
        let rest = &text[end..];
        let mut result = vec![part.trim().to_string()];
        result.extend(split_recursive_base(rest, separators, chunk_size));
        return result;
    };

    let sep = &separators[separator_index];
    let next_separators = &separators[separator_index + 1..];

    let mut pieces = Vec::new();
    let mut current = String::new();

    for segment in text.split(sep.as_str()) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let candidate = if current.is_empty() {
            segment.to_string()
        } else {
            format!("{current}{sep}{segment}")
        };
        if candidate.chars().count() > chunk_size && !current.is_empty() {
            pieces.push(current.clone());
            current = segment.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }

    let mut result = Vec::new();
    for piece in pieces {
        if piece.chars().count() <= chunk_size || next_separators.is_empty() {
            result.push(piece);
        } else {
            result.extend(split_recursive_base(&piece, next_separators, chunk_size));
        }
    }

    result
}

/// Chunking recursivo: segmentação base pelo maior separador + cauda de overlap.
///
/// Quando `overlap` é maior que zero, cada pedaço recebe um prefixo do pedaço
/// seguinte como contexto extra.
pub(crate) fn split_recursive(
    text: &str,
    separators: &[String],
    chunk_size: usize,
    overlap: usize,
) -> Result<Vec<String>, ChunkError> {
    if text.is_empty() {
        return Err(ChunkError::EmptyDocument);
    }

    let base = split_recursive_base(text.trim(), separators, chunk_size);
    if overlap == 0 {
        return Ok(base);
    }

    let mut chunks = Vec::with_capacity(base.len());
    for (i, piece) in base.iter().enumerate() {
        let mut chunk = piece.clone();
        if i + 1 < base.len() {
            let tail: String = base[i + 1].chars().take(overlap).collect();
            chunk.push_str(&tail);
        }
        chunks.push(chunk);
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_recursive_preserves_order() {
        let separators = vec!["\n\n".to_string(), " ".to_string()];
        let text = "primeiro segundo terceiro quarto".to_string();
        let chunks = split_recursive(&text, &separators, 8, 0).unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| !c.is_empty()));
    }

    #[test]
    fn split_fixed_respects_size() {
        let pieces = split_fixed("palavra palavra palavra palavra palavra", 10, 0);
        assert!(!pieces.is_empty());
        assert!(pieces.iter().all(|p| p.chars().count() <= 11));
    }

    #[test]
    fn split_fixed_overlap_shares_context() {
        let pieces = split_fixed(&"abcdefghij".repeat(5), 10, 4);
        assert!(pieces.len() > 1);
    }
}
