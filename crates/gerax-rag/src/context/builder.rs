use crate::vector::VectorSearchResult;

use super::{ContextBuilder, ContextError};

/// Configuração do [`SimpleContextBuilder`].
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// Limite de caracteres do contexto montado.
    pub max_characters: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_characters: 8192,
        }
    }
}

/// Builder de contexto simples que monta blocos `[Source: <id>]` por resultado.
///
/// Concatena os resultados na ordem recebida, respeitando o limite de
/// caracteres configurado em [`ContextConfig::max_characters`].
pub struct SimpleContextBuilder {
    config: ContextConfig,
}

impl Default for SimpleContextBuilder {
    fn default() -> Self {
        Self::new(ContextConfig::default())
    }
}

impl SimpleContextBuilder {
    /// Cria o builder com a configuração dada.
    pub fn new(config: ContextConfig) -> Self {
        Self { config }
    }

    /// Substitui a configuração, encadeável.
    pub fn with_config(mut self, config: ContextConfig) -> Self {
        self.config = config;
        self
    }
}

impl ContextBuilder for SimpleContextBuilder {
    fn build(&self, results: &[VectorSearchResult]) -> Result<String, ContextError> {
        if results.is_empty() {
            return Err(ContextError::EmptyResults);
        }

        let mut context = String::new();
        let mut chars_left = self.config.max_characters;

        for (i, result) in results.iter().enumerate() {
            let header = format!("[Source: {}]", result.document.id);
            let content = &result.document.content;

            let mut block = String::new();
            if i > 0 {
                block.push_str("\n\n---\n\n");
            }
            block.push_str(&header);
            block.push('\n');
            block.push('\n');
            block.push_str(content);

            let block_chars = block.chars().count();
            if block_chars > chars_left {
                break;
            }

            context.push_str(&block);
            chars_left -= block_chars;
        }

        Ok(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::Embedding;
    use crate::vector::{VectorDocument, VectorSearchResult};

    fn result(id: &str, content: &str) -> VectorSearchResult {
        VectorSearchResult {
            document: VectorDocument {
                id: id.to_string(),
                vector: Embedding {
                    vector: vec![1.0, 0.0],
                },
                content: content.to_string(),
                metadata: Default::default(),
            },
            score: 1.0,
        }
    }

    #[test]
    fn empty_results_fails() {
        let builder = SimpleContextBuilder::default();
        let result = builder.build(&[]);
        assert!(matches!(result, Err(ContextError::EmptyResults)));
    }

    #[test]
    fn builds_source_blocks() {
        let builder = SimpleContextBuilder::default();
        let context = builder
            .build(&[result("doc-a", "conteudo a"), result("doc-b", "conteudo b")])
            .unwrap();
        assert!(context.contains("[Source: doc-a]"));
        assert!(context.contains("conteudo a"));
        assert!(context.contains("[Source: doc-b]"));
        assert!(context.contains("conteudo b"));
        assert!(context.contains("---"));
    }

    #[test]
    fn respects_max_characters() {
        let config = ContextConfig { max_characters: 40 };
        let builder = SimpleContextBuilder::new(config);
        let context = builder
            .build(&[result("doc-a", "conteudo a"), result("doc-b", "conteudo b")])
            .unwrap();
        assert!(context.chars().count() <= 40);
        // doc-b block should be cut off
        assert!(!context.contains("[Source: doc-b]"));
    }

    #[test]
    fn stops_before_exceeding_limit() {
        let config = ContextConfig { max_characters: 40 };
        let builder = SimpleContextBuilder::new(config);
        let context = builder
            .build(&[result("doc-a", "conteudo a"), result("doc-b", "conteudo b")])
            .unwrap();
        assert!(context.chars().count() <= 40);
        assert!(context.contains("conteudo a"));
        assert!(!context.contains("conteudo b"));
    }
}
