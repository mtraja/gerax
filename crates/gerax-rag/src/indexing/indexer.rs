use std::sync::Arc;

use crate::chunking::{ChunkRequest, Chunker};
use crate::document::Document;
use crate::embedding::EmbeddingProvider;
use crate::vector::{VectorDocument, VectorStore};

use super::{IndexError, IndexingResult};

/// Indexa documentos transformando-os em chunks, embeddings e vetores.
///
/// Orquestra três estágios: o [`Chunker`] divide o documento, o
/// [`EmbeddingProvider`] gera os vetores e o [`VectorStore`] persiste os
/// resultados. Prefira construir via [`DocumentIndexer::builder`].
pub struct DocumentIndexer {
    chunker: Arc<dyn Chunker>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    vector_store: Arc<dyn VectorStore>,
}

impl DocumentIndexer {
    /// Inicia a construção do indexador via `DocumentIndexerBuilder`.
    pub fn builder() -> DocumentIndexerBuilder {
        DocumentIndexerBuilder::default()
    }

    /// Cria o indexador com as dependências já resolvidas.
    pub fn new(
        chunker: Arc<dyn Chunker>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        Self {
            chunker,
            embedding_provider,
            vector_store,
        }
    }

    /// Indexa um documento de ponta a ponta (chunking, embedding, storage).
    ///
    /// Retorna um [`IndexingResult`] com a contagem de chunks criados e
    /// persistidos. Emite eventos de `tracing::debug` por estágio.
    pub async fn index(&self, document: Document) -> Result<IndexingResult, IndexError> {
        let document_id = document.id.clone();
        tracing::debug!(document_id = %document_id, "indexing document");

        let chunks = self.chunker.chunk(ChunkRequest { document })?;
        let chunks_created = chunks.len();
        tracing::debug!(
            document_id = %document_id,
            chunks_created,
            "document chunked"
        );

        let contents: Vec<String> = chunks.iter().map(|chunk| chunk.content.clone()).collect();
        let embeddings = self.embedding_provider.embed_batch(&contents).await?;
        tracing::debug!(document_id = %document_id, total = embeddings.len(), "embedding generated");

        for (chunk, embedding) in chunks.into_iter().zip(embeddings) {
            let vector_document = VectorDocument {
                id: chunk.id.clone(),
                vector: embedding,
                content: chunk.content,
                metadata: chunk.metadata,
            };
            tracing::debug!(chunk_id = %vector_document.id, "vector stored");
            self.vector_store.upsert(vector_document).await?;
        }

        Ok(IndexingResult {
            document_id,
            chunks_created,
            chunks_indexed: chunks_created,
        })
    }
}

/// Builder incremental para [`DocumentIndexer`].
///
/// Valida a presença de `chunker`, `embedding_provider` e `vector_store` antes
/// de construir o indexador.
#[derive(Default)]
pub struct DocumentIndexerBuilder {
    chunker: Option<Arc<dyn Chunker>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    vector_store: Option<Arc<dyn VectorStore>>,
}

impl DocumentIndexerBuilder {
    /// Define o [`Chunker`] a ser usado.
    pub fn chunker(mut self, chunker: Arc<dyn Chunker>) -> Self {
        self.chunker = Some(chunker);
        self
    }

    /// Define o [`EmbeddingProvider`] a ser usado.
    pub fn embedding_provider(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.embedding_provider = Some(provider);
        self
    }

    /// Define o [`VectorStore`] a ser usado.
    pub fn vector_store(mut self, store: Arc<dyn VectorStore>) -> Self {
        self.vector_store = Some(store);
        self
    }

    /// Constrói o [`DocumentIndexer`], exigindo todas as dependências.
    ///
    /// Falha com [`IndexError::MissingField`] quando alguma dependência estiver
    /// ausente.
    pub fn build(self) -> Result<DocumentIndexer, IndexError> {
        Ok(DocumentIndexer {
            chunker: self
                .chunker
                .ok_or_else(|| IndexError::MissingField("chunker".into()))?,
            embedding_provider: self
                .embedding_provider
                .ok_or_else(|| IndexError::MissingField("embedding_provider".into()))?,
            vector_store: self
                .vector_store
                .ok_or_else(|| IndexError::MissingField("vector_store".into()))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunking::FixedSizeChunker;
    use crate::embedding::MockEmbeddingProvider;
    use crate::vector::InMemoryVectorStore;

    fn deps() -> (
        Arc<dyn Chunker>,
        Arc<dyn EmbeddingProvider>,
        Arc<dyn VectorStore>,
    ) {
        (
            Arc::new(FixedSizeChunker::new(32, 4).unwrap()),
            Arc::new(MockEmbeddingProvider::default()),
            Arc::new(InMemoryVectorStore::new()),
        )
    }

    #[tokio::test]
    async fn builder_missing_field_fails() {
        let (chunker, provider, _) = deps();
        assert!(matches!(
            DocumentIndexer::builder().chunker(chunker).build(),
            Err(IndexError::MissingField(ref field)) if field == "embedding_provider"
        ));
        assert!(matches!(
            DocumentIndexer::builder()
                .chunker(Arc::new(FixedSizeChunker::new(10, 2).unwrap()))
                .embedding_provider(provider)
                .build(),
            Err(IndexError::MissingField(ref field)) if field == "vector_store"
        ));
    }

    #[tokio::test]
    async fn index_document_stores_chunks() {
        let (chunker, provider, store) = deps();
        let indexer = DocumentIndexer::builder()
            .chunker(chunker.clone())
            .embedding_provider(provider.clone())
            .vector_store(store.clone())
            .build()
            .unwrap();

        let content = "Para cancelar uma matricula o aluno deve acessar o portal. \
            Preencha o formulario de cancelamento. O cancelamento so pode ser feito pelo titular."
            .repeat(4);

        let doc = Document::builder()
            .id("manual")
            .content(content)
            .metadata("source", "manual")
            .build()
            .unwrap();

        let result = indexer.index(doc).await.unwrap();
        assert_eq!(result.document_id, "manual");
        assert!(result.chunks_created > 0);
        assert_eq!(result.chunks_created, result.chunks_indexed);

        let search = crate::vector::VectorSearchRequest {
            vector: provider.embed("manual portal aluno").await.unwrap(),
            limit: result.chunks_created,
            min_score: None,
            filter: None,
        };
        let found = store.search(search).await.unwrap();
        assert_eq!(found.len(), result.chunks_created);
    }
}
