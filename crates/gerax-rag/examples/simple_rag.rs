use std::sync::Arc;

use gerax_rag::chunking::FixedSizeChunker;
use gerax_rag::context::{ContextConfig, SimpleContextBuilder};
use gerax_rag::document::Document;
use gerax_rag::embedding::MockEmbeddingProvider;
use gerax_rag::indexing::DocumentIndexer;
use gerax_rag::pipeline::{RagPipeline, RagRequest};
use gerax_rag::retrieval::Retriever;
use gerax_rag::vector::InMemoryVectorStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chunker = Arc::new(FixedSizeChunker::new(96, 16).unwrap());
    let embeddings = Arc::new(MockEmbeddingProvider::default());
    let store = Arc::new(InMemoryVectorStore::new());

    let indexer = DocumentIndexer::builder()
        .chunker(chunker)
        .embedding_provider(embeddings.clone())
        .vector_store(store.clone())
        .build()?;

    let content = "\
Para cancelar uma matricula o aluno deve acessar o portal academico.
Preencha o formulario de cancelamento de matricula informando o motivo.
O cancelamento so pode ser realizado pelo titular da matricula.
Apos a confirmacao, o sistema envia um email de confirmacao para o aluno.
O prazo para cancelamento sem multa eh de trinta dias apos o inicio do semestre.
";

    indexer
        .index(
            Document::builder()
                .id("manual-alunos")
                .content(content)
                .metadata("source", "manual")
                .metadata("language", "pt-BR")
                .build()?,
        )
        .await?;

    let retriever = Retriever::new(embeddings, store);
    let pipeline = RagPipeline::builder()
        .retriever(retriever)
        .context_builder(Arc::new(
            SimpleContextBuilder::new(ContextConfig::default()),
        ))
        .build()?;

    let result = pipeline
        .run(RagRequest {
            query: "como desfazer a inscricao?".to_string(),
            retrieval_limit: 3,
            min_score: None,
            filter: None,
        })
        .await?;

    println!("Query: {}", result.query);
    println!("--- Context ---");
    println!("{}", result.context);

    for document in &result.documents {
        println!("\n[{}] score={:.4}", document.document.id, document.score);
        println!("{}", document.document.content);
    }

    Ok(())
}
