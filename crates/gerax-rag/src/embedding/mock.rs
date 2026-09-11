use async_trait::async_trait;

use super::{Embedding, EmbeddingError, EmbeddingProvider};

const DEFAULT_DIMENSIONS: usize = 512;
const MODEL_NAME: &str = "mock";

/// Provider de embeddings determinístico, usado em testes e exemplos.
///
/// Gera vetores baseados em hash estável (FNV-1a) dos tokens do texto,
/// normalizados por norma L2. Textos que compartilham tokens produzem vetores
/// com similaridade cosseno positiva.
pub struct MockEmbeddingProvider {
    dimensions: usize,
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self {
            dimensions: DEFAULT_DIMENSIONS,
        }
    }
}

impl MockEmbeddingProvider {
    /// Cria um provider com a dimensão informada.
    ///
    /// Valores `0` caem na dimensão padrão (`DEFAULT_DIMENSIONS`).
    pub fn new(dimensions: usize) -> Self {
        let dimensions = if dimensions == 0 {
            DEFAULT_DIMENSIONS
        } else {
            dimensions
        };
        Self { dimensions }
    }

    /// Altera a dimensão do provider encadeadamente.
    pub fn with_dimensions(mut self, dimensions: usize) -> Self {
        if dimensions != 0 {
            self.dimensions = dimensions;
        }
        self
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    fn model(&self) -> &str {
        MODEL_NAME
    }

    fn dimensions(&self) -> Option<usize> {
        Some(self.dimensions)
    }

    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        if text.is_empty() {
            return Err(EmbeddingError::EmptyText);
        }
        Ok(embed_text(text, self.dimensions))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Embedding>, EmbeddingError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
}

/// Gera o vetor de embeddings determinístico para `text` com a dimensão dada.
fn embed_text(text: &str, dimensions: usize) -> Embedding {
    let mut vector = vec![0.0f32; dimensions];

    for token in text.split(|c: char| !c.is_alphanumeric()) {
        let token = token.trim().to_lowercase();
        if token.is_empty() {
            continue;
        }
        let idx_a = stable_hash(&token) % dimensions;
        let idx_b = stable_hash(&format!("salt:{}", token)) % dimensions;
        vector[idx_a] += 1.0;
        vector[idx_b] += 1.0;
    }

    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm == 0.0 {
        return Embedding { vector };
    }

    for value in &mut vector {
        *value /= norm;
    }

    Embedding { vector }
}

/// Hash FNV-1a estável de uma string (determinístico entre execuções).
fn stable_hash(token: &str) -> usize {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in token.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn embedding_dimensions() {
        let provider = MockEmbeddingProvider::new(8);
        let embedding = provider.embed("hello").await.unwrap();
        assert_eq!(embedding.dimensions(), 8);
        assert_eq!(provider.dimensions(), Some(8));
        assert_eq!(provider.model(), "mock");
    }

    #[tokio::test]
    async fn embedding_is_deterministic() {
        let provider = MockEmbeddingProvider::default();
        let a = provider.embed("alguma frase interessante").await.unwrap();
        let b = provider.embed("alguma frase interessante").await.unwrap();
        assert_eq!(a.vector, b.vector);
    }

    #[tokio::test]
    async fn empty_text_fails() {
        let provider = MockEmbeddingProvider::default();
        let result = provider.embed("").await;
        assert!(matches!(result, Err(EmbeddingError::EmptyText)));
    }

    #[tokio::test]
    async fn batch_preserves_order_and_count() {
        let provider = MockEmbeddingProvider::new(16);
        let texts = vec!["um".to_string(), "dois".to_string(), "tres".to_string()];
        let embeddings = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 3);
        assert_eq!(
            embeddings[0].vector,
            provider.embed("um").await.unwrap().vector
        );
    }

    #[tokio::test]
    async fn shared_tokens_produce_similar_vectors() {
        let provider = MockEmbeddingProvider::default();
        let a = provider.embed("matricula de alunos").await.unwrap();
        let b = provider.embed("matricula e transferencia").await.unwrap();
        let c = provider
            .embed("importar arquivos de exportacao")
            .await
            .unwrap();

        let sim_ab = cosine(&a.vector, &b.vector);
        let sim_ac = cosine(&a.vector, &c.vector);
        assert!(
            sim_ab > sim_ac,
            "sim(shared)={sim_ab} sim(distinct)={sim_ac}"
        );
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm_a * norm_b)
    }
}
