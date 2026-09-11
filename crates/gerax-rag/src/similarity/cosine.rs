use super::{SimilarityError, SimilarityMetric};

/// Similaridade por cosseno entre vetores normalizados.
#[derive(Debug, Default, Clone)]
pub struct CosineSimilarity;

impl CosineSimilarity {
    /// Cria uma nova instância de similaridade por cosseno.
    pub fn new() -> Self {
        Self
    }
}

/// Calcula a similaridade por cosseno entre dois vetores.
///
/// Calculada com precisão dupla internamente para reduzir erros de ponto
/// flutuante. Falha com [`SimilarityError::EmptyVector`] para vetores vazios,
/// [`SimilarityError::DimensionMismatch`] para dimensões diferentes e
/// [`SimilarityError::ZeroNorm`] quando algum vetor tem norma nula.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, SimilarityError> {
    if a.is_empty() || b.is_empty() {
        return Err(SimilarityError::EmptyVector);
    }
    if a.len() != b.len() {
        return Err(SimilarityError::DimensionMismatch(a.len(), b.len()));
    }

    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += f64::from(*x) * f64::from(*y);
        norm_a += f64::from(*x) * f64::from(*x);
        norm_b += f64::from(*y) * f64::from(*y);
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return Err(SimilarityError::ZeroNorm);
    }

    Ok((dot / (norm_a.sqrt() * norm_b.sqrt())) as f32)
}

impl SimilarityMetric for CosineSimilarity {
    fn similarity(&self, a: &[f32], b: &[f32]) -> Result<f32, SimilarityError> {
        cosine_similarity(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_vectors_similarity_one() {
        let v = [0.5, 0.5, 0.707];
        let sim = cosine_similarity(&v, &v).unwrap();
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn opposite_vectors_similarity_minus_one() {
        let v = [1.0, 0.0];
        let neg = [-1.0, 0.0];
        let sim = cosine_similarity(&v, &neg).unwrap();
        assert!((sim + 1.0).abs() < 1e-5);
    }

    #[test]
    fn orthogonal_vectors_similarity_zero() {
        let a = [1.0, 0.0];
        let b = [0.0, 1.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn dimension_mismatch_fails() {
        let a = [1.0, 2.0];
        let b = [1.0];
        assert!(matches!(
            cosine_similarity(&a, &b),
            Err(SimilarityError::DimensionMismatch(2, 1))
        ));
    }

    #[test]
    fn empty_vector_fails() {
        let empty: [f32; 0] = [];
        let v = [1.0];
        assert!(matches!(
            cosine_similarity(&empty, &v),
            Err(SimilarityError::EmptyVector)
        ));
        assert!(matches!(
            cosine_similarity(&v, &empty),
            Err(SimilarityError::EmptyVector)
        ));
    }

    #[test]
    fn zero_vector_fails() {
        let zero = [0.0, 0.0];
        let v = [1.0, 0.0];
        assert!(matches!(
            cosine_similarity(&zero, &v),
            Err(SimilarityError::ZeroNorm)
        ));
        assert!(matches!(
            cosine_similarity(&v, &zero),
            Err(SimilarityError::ZeroNorm)
        ));
    }

    #[test]
    fn parallel_vectors_in_same_direction_positive() {
        let a = [1.0, 2.0, 3.0];
        let b = [2.0, 4.0, 6.0];
        let sim = cosine_similarity(&a, &b).unwrap();
        assert!((sim - 1.0).abs() < 1e-5);
    }
}
