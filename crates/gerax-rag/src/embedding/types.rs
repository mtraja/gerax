use async_trait::async_trait;
use thiserror::Error;

/// Erros possíveis ao gerar embeddings.
#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("cannot embed empty text")]
    EmptyText,
    #[error("embedding provider failed: {0}")]
    Provider(String),
}

/// Vetor numérico obtido ao converter texto em representação semântica.
#[derive(Debug, Clone)]
pub struct Embedding {
    /// Valores do vetor, normalmente normalizados por norma L2.
    pub vector: Vec<f32>,
}

impl Embedding {
    /// Dimensão do vetor (`vector.len()`).
    pub fn dimensions(&self) -> usize {
        self.vector.len()
    }
}

/// Abstração sobre um provider de embeddings (Ollama, OpenAI, etc.).
///
/// Implementações específicas devem ser fornecidas em crates separadas que
/// implementem este trait e sejam usadas no pipeline via `Arc<dyn EmbeddingProvider>`.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn model(&self) -> &str;

    fn dimensions(&self) -> Option<usize>;

    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Embedding>, EmbeddingError>;
}
