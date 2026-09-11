//! Núcleo de Retrieval-Augmented Generation (RAG) e busca semântica do ecossistema Gerax.
//!
//! A crate fornece um pipeline RAG completo e desacoplado de providers externos:
//!
//! - [`document`]: modelo de documentos com metadados;
//! - [`chunking`]: divisão de documentos em pedaços (`Chunker`);
//! - [`embedding`]: geração de embeddings via [`EmbeddingProvider`];
//! - [`vector`]: armazenamento vetorial e busca por similaridade ([`VectorStore`]);
//! - [`indexing`]: orquestração da indexação de documentos;
//! - [`retrieval`]: recuperação de trechos relevantes à consulta;
//! - [`reranking`]: reordenação dos resultados recuperados;
//! - [`context`]: montagem do contexto enviado ao LLM;
//! - [`pipeline`]: orquestração do fluxo RAG de ponta a ponta.
//!
//! A arquitetura segue o padrão *ports & adapters*: as abstrações (traits) são a API
//! pública, enquanto implementações específicas de providers (Ollama, Qdrant, etc.)
//! devem viver em crates separadas.
//!
//! # Exemplo
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use gerax_rag::chunking::FixedSizeChunker;
//! use gerax_rag::context::{ContextConfig, SimpleContextBuilder};
//! use gerax_rag::document::Document;
//! use gerax_rag::embedding::MockEmbeddingProvider;
//! use gerax_rag::indexing::DocumentIndexer;
//! use gerax_rag::pipeline::{RagPipeline, RagRequest};
//! use gerax_rag::retrieval::Retriever;
//! use gerax_rag::vector::InMemoryVectorStore;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let chunker = Arc::new(FixedSizeChunker::new(256, 32)?);
//!     let embeddings = Arc::new(MockEmbeddingProvider::default());
//!     let store = Arc::new(InMemoryVectorStore::new());
//!
//!     let indexer = DocumentIndexer::builder()
//!         .chunker(chunker)
//!         .embedding_provider(embeddings.clone())
//!         .vector_store(store.clone())
//!         .build()?;
//!
//!     indexer
//!         .index(
//!             Document::builder()
//!                 .id("manual")
//!                 .content("Para cancelar uma matrícula acesse o portal acadêmico.")
//!                 .build()?,
//!         )
//!         .await?;
//!
//!     let retriever = Retriever::new(embeddings, store);
//!     let pipeline = RagPipeline::builder()
//!         .retriever(retriever)
//!         .context_builder(Arc::new(SimpleContextBuilder::new(ContextConfig::default())))
//!         .build()?;
//!
//!     let result = pipeline
//!         .run(RagRequest::new("como cancelar a matrícula?", 3))
//!         .await?;
//!
//!     println!("{}", result.context);
//!     Ok(())
//! }
//! ```

pub mod chunking;
pub mod context;
pub mod document;
pub mod embedding;
pub mod indexing;
pub mod pipeline;
pub mod query;
pub mod reranking;
pub mod retrieval;
pub mod similarity;
pub mod vector;

pub use chunking::{
    Chunk, ChunkError, ChunkRequest, Chunker, FixedSizeChunker, FixedSizeChunkerError,
    MarkdownChunker, MarkdownChunkerError, RecursiveChunker, RecursiveChunkerError,
};
pub use context::{ContextBuilder, ContextConfig, ContextError, SimpleContextBuilder};
pub use document::{Document, DocumentBuilder, DocumentError, DocumentMetadata};
pub use embedding::{Embedding, EmbeddingError, EmbeddingProvider, MockEmbeddingProvider};
pub use indexing::{DocumentIndexer, IndexError, IndexingResult};
pub use pipeline::{RagError, RagPipeline, RagRequest, RagResult};
pub use query::{Query, QueryError};
pub use reranking::{NoopReranker, Reranker, RerankerError};
pub use retrieval::{RetrievalError, RetrievalRequest, RetrievalResult, Retriever};
pub use similarity::{CosineSimilarity, SimilarityError, SimilarityMetric};
pub use vector::{
    Filter, InMemoryVectorStore, VectorDocument, VectorFilter, VectorSearchRequest,
    VectorSearchResult, VectorStore, VectorStoreError,
};
