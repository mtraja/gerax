use thiserror::Error;

use crate::vector::VectorSearchResult;

/// Erros possíveis ao montar o contexto.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Nenhum resultado foi fornecido ao builder de contexto.
    #[error("empty results provided to context builder")]
    EmptyResults,
}

/// Monta o texto de contexto a partir dos resultados recuperados.
///
/// O contexto é a porção do prompt do LLM que contém o conhecimento recuperado.
pub trait ContextBuilder: Send + Sync {
    /// Constrói o contexto textual a partir dos resultados.
    fn build(&self, results: &[VectorSearchResult]) -> Result<String, ContextError>;
}
