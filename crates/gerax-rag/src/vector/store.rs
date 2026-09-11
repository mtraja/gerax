use async_trait::async_trait;

use super::{VectorDocument, VectorSearchRequest, VectorSearchResult, VectorStoreError};

/// Abstração sobre um armazenamento vetorial (em memória, Qdrant, etc.).
///
/// Implementações específicas devem ser fornecidas em crates separadas e usadas
/// via `Arc<dyn VectorStore>`.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Insere ou atualiza um documento vetorial pelo `id`.
    async fn upsert(&self, document: VectorDocument) -> Result<(), VectorStoreError>;

    /// Busca um documento vetorial pelo `id`, retornando `None` se ausente.
    async fn get(&self, id: &str) -> Result<Option<VectorDocument>, VectorStoreError>;

    /// Remove um documento vetorial pelo `id`. Não falha se ausente.
    async fn delete(&self, id: &str) -> Result<(), VectorStoreError>;

    /// Busca os documentos mais similares ao vetor da consulta.
    ///
    /// Resultados são retornados ordenados por score decrescente e limitados a
    /// `request.limit`.
    async fn search(
        &self,
        request: VectorSearchRequest,
    ) -> Result<Vec<VectorSearchResult>, VectorStoreError>;
}
