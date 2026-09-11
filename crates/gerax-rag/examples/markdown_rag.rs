use std::sync::Arc;

use gerax_rag::chunking::MarkdownChunker;
use gerax_rag::context::{ContextConfig, SimpleContextBuilder};
use gerax_rag::document::Document;
use gerax_rag::embedding::MockEmbeddingProvider;
use gerax_rag::indexing::DocumentIndexer;
use gerax_rag::pipeline::{RagPipeline, RagRequest};
use gerax_rag::retrieval::Retriever;
use gerax_rag::vector::InMemoryVectorStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let chunker = Arc::new(MarkdownChunker::default());
    let embeddings = Arc::new(MockEmbeddingProvider::default());
    let store = Arc::new(InMemoryVectorStore::new());

    let indexer = DocumentIndexer::builder()
        .chunker(chunker)
        .embedding_provider(embeddings.clone())
        .vector_store(store.clone())
        .build()?;

    let markdown = "\
# Matrícula

## Cancelamento

Para cancelar uma matrícula o aluno deve acessar o portal acadêmico.
Preencha o formulário de cancelamento informando o motivo da solicitação.

## Trava de segurança

O cancelamento só pode ser realizado pelo titular da matrícula.
Após a confirmação, o sistema envia um e-mail para o aluno.

# Calendário

## Início das aulas

O semestre começa no primeiro dia de março.

## Provas

A prova final ocorre na segunda quinzena de julho.
";

    indexer
        .index(
            Document::builder()
                .id("manual-universidade")
                .content(markdown)
                .metadata("repository", "escola-docs")
                .metadata("language", "pt-BR")
                .build()?,
        )
        .await?;

    let retriever = Retriever::new(embeddings.clone(), store);
    let pipeline = RagPipeline::builder()
        .retriever(retriever)
        .context_builder(Arc::new(
            SimpleContextBuilder::new(ContextConfig::default()),
        ))
        .build()?;

    let result = pipeline
        .run(RagRequest {
            query: "como cancelar a matricula de um aluno?".to_string(),
            retrieval_limit: 3,
            min_score: None,
            filter: None,
        })
        .await?;

    println!("Query: {}", result.query);
    println!("--- Context ---");
    println!("{}", result.context);
    println!("---");

    for hit in &result.documents {
        println!("score={:.4} id={}", hit.score, hit.document.id);
        let path = hit
            .document
            .metadata
            .get("heading_path")
            .and_then(|v| v.as_str())
            .unwrap_or("(sem seção)");
        println!("seção: {path}");
    }

    Ok(())
}
