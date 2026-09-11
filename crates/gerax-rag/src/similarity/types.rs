use thiserror::Error;

/// Erros possíveis ao calcular similaridade entre vetores.
#[derive(Debug, Error)]
pub enum SimilarityError {
    /// Um dos vetores está vazio.
    #[error("vectors cannot be empty")]
    EmptyVector,
    /// Os vetores possuem dimensões diferentes.
    #[error("vectors must have the same dimensions (got {0} and {1})")]
    DimensionMismatch(usize, usize),
    /// Um dos vetores tem norma zero e não pode ser comparado.
    #[error("vector has zero norm")]
    ZeroNorm,
}

/// Métrica de similaridade entre dois vetores, retornando uma pontuação.
pub trait SimilarityMetric: Send + Sync {
    /// Calcula a similaridade entre `a` e `b` (ex.: cosseno).
    fn similarity(&self, a: &[f32], b: &[f32]) -> Result<f32, SimilarityError>;
}
