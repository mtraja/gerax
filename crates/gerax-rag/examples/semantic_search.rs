use std::sync::Arc;

use gerax_rag::chunking::{ChunkRequest, Chunker, FixedSizeChunker};
use gerax_rag::document::Document;
use gerax_rag::embedding::{EmbeddingProvider, MockEmbeddingProvider};
use gerax_rag::retrieval::{RetrievalRequest, Retriever};
use gerax_rag::vector::{
    Filter, InMemoryVectorStore, VectorDocument, VectorFilter, VectorSearchRequest, VectorStore,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let embeddings = Arc::new(MockEmbeddingProvider::default());
    let store = Arc::new(InMemoryVectorStore::new());
    let chunker = FixedSizeChunker::new(128, 8).unwrap();

    let entries: Vec<(String, String, &str)> = vec![
        (
            "manual-matricula".into(),
            "Para cancelar uma matricula o aluno deve acessar o portal academico e preencher o formulario de cancelamento.".into(),
            "rust",
        ),
        (
            "manual-calendario".into(),
            "O semestre comeca em primeiro de marco e a prova final ocorre em quinze de julho.".into(),
            "rust",
        ),
        (
            "manual-biblioteca".into(),
            "Emprestimos de livros podem ser renovados duas vezes pelo portal da biblioteca.".into(),
            "rust",
        ),
        (
            "manual-frequencia".into(),
            "A frequencia minima exigida para aprovacao e de setenta e cinco por cento.".into(),
            "rust",
        ),
    ];

    for (id, content, language) in entries {
        let document = Document::builder()
            .id(&id)
            .content(&content)
            .metadata("repository", "escola-docs")
            .metadata("language", language)
            .build()?;

        let chunks = chunker.chunk(ChunkRequest { document }).unwrap();
        for chunk in chunks {
            let vector = embeddings.embed(&chunk.content).await?;
            store
                .upsert(VectorDocument {
                    id: chunk.id,
                    vector,
                    content: chunk.content,
                    metadata: chunk.metadata,
                })
                .await?;
        }
    }

    let retriever = Retriever::new(embeddings.clone(), store.clone());

    println!("== Busca sem filtro ==");
    let all = retriever
        .retrieve(RetrievalRequest {
            query: "cancelar matricula".into(),
            limit: 4,
            min_score: None,
            filter: None,
        })
        .await?;
    for result in all.results {
        println!("  {} ({:.4})", result.document.id, result.score);
    }

    println!("\n== Busca com filtro repository=escola-docs ==");
    let filtered = store
        .search(VectorSearchRequest {
            vector: embeddings.embed("cancelar matricula").await?,
            limit: 4,
            min_score: None,
            filter: Some(VectorFilter::and([Filter::eq("repository", "escola-docs")])),
        })
        .await?;
    for result in filtered {
        println!("  {} ({:.4})", result.document.id, result.score);
    }

    Ok(())
}
