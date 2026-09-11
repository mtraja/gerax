use std::sync::Arc;

use gerax_rag::chunking::FixedSizeChunker;
use gerax_rag::document::Document;
use gerax_rag::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use gerax_rag::indexing::DocumentIndexer;
use gerax_rag::vector::{InMemoryVectorStore, VectorSearchRequest, VectorStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chunker = Arc::new(FixedSizeChunker::new(64, 8).unwrap());
    let embeddings = Arc::new(MockEmbeddingProvider::default());
    let store = Arc::new(InMemoryVectorStore::new());

    let indexer = DocumentIndexer::builder()
        .chunker(chunker)
        .embedding_provider(embeddings.clone())
        .vector_store(store.clone())
        .build()?;

    let content = "\
Regras de matricula. Para cancelar uma matricula o aluno deve acessar o portal academico.
O formulario de cancelamento exige o motivo da solicitacao.
O cancelamento so pode ser feito pelo titular.
Calendario. As aulas comecam em primeiro de marco.
A prova final ocorre em quinze de julho.
";

    let result = indexer
        .index(
            Document::builder()
                .id("manual")
                .content(content)
                .metadata("source", "school-manual")
                .build()?,
        )
        .await?;

    println!(
        "Indexado: document_id={} chunks_created={} chunks_indexed={}",
        result.document_id, result.chunks_created, result.chunks_indexed
    );

    let query_embedding = embeddings.embed("data da prova final").await?;
    let results = store
        .search(VectorSearchRequest {
            vector: query_embedding,
            limit: 3,
            min_score: None,
            filter: None,
        })
        .await?;

    for result in results {
        println!("---");
        println!("chunk: {}", result.document.id);
        println!("score: {:.4}", result.score);
        println!("{}", result.document.content);
    }

    Ok(())
}
