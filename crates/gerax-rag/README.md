# gerax-rag

Núcleo de **Retrieval-Augmented Generation (RAG)** e busca semântica do ecossistema
Gerax. A crate fornece um pipeline RAG completo e **desacoplado de providers
externos**, seguindo o padrão *ports & adapters*: as abstrações (traits) são a
API pública, enquanto implementações específicas (Ollama, Qdrant, OpenAI etc.)
devem viver em crates separadas.

## Características

- Pipeline RAG de ponta a ponta: recuperação → reranking → contexto.
- 100% assíncrono (`tokio` + `async_trait`).
- Arquitetura *ports & adapters* (trait objects com `Arc<dyn ...>`), pronta
  para receber providers reais sem mudanças na API.
- Implementações de referência incluídas: chunkers, re-ranker no-op, métrica
  cosseno, vector store em memória e provider de embeddings determinístico
  (testes e protótipos).
- Filtros de metadados na recuperação (`Filter::eq` / `Filter::in_list`).
- Observabilidade via `tracing` (spans e eventos de debug por estágio).
- Tipagem rígida de erros com `thiserror`.

## Módulos

| Módulo | Responsabilidade |
| --- | --- |
| [`document`](src/document) | Modelo de documento: `id`, `content`, metadados e `DocumentBuilder`. |
| [`chunking`](src/chunking) | Divisão de documentos em chunks (`Chunker`): `FixedSizeChunker`, `RecursiveChunker` e `MarkdownChunker`. |
| [`embedding`](src/embedding) | Geração de embeddings (`EmbeddingProvider`); `MockEmbeddingProvider` determinístico. |
| [`vector`](src/vector) | Armazenamento vetorial (`VectorStore`), busca por similaridade e filtros (`Filter`/`VectorFilter`). |
| [`indexing`](src/indexing) | Orquestração da indexação: `DocumentIndexer` (chunking → embedding → storage). |
| [`retrieval`](src/retrieval) | Recuperação semântica (`Retriever`) com limite, score mínimo e filtro. |
| [`reranking`](src/reranking) | Reordenação dos resultados (`Reranker`); `NoopReranker` preserva a ordem. |
| [`context`](src/context) | Montagem do contexto do LLM (`ContextBuilder`); `SimpleContextBuilder`. |
| [`pipeline`](src/pipeline) | Orquestração RAG ponta a ponta (`RagPipeline`). |
| [`query`](src/query) | Consulta validada (`Query`), texto não vazio. |
| [`similarity`](src/similarity) | Métricas de similaridade (`SimilarityMetric`); cosseno. |

## Fluxo

### Indexação

```
Documento ──► Chunker ──► Chunks ──► EmbeddingProvider ──► VectorStore
```

### Consulta (RAG)

```
Query ──► Retriever (embed + VectorStore.search) ──► [Reranker] ──► ContextBuilder ──► Contexto p/ LLM
```

## Exemplo mínimo

```rust,no_run
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
    let chunker = Arc::new(FixedSizeChunker::new(256, 32)?);
    let embeddings = Arc::new(MockEmbeddingProvider::default());
    let store = Arc::new(InMemoryVectorStore::new());

    let indexer = DocumentIndexer::builder()
        .chunker(chunker)
        .embedding_provider(embeddings.clone())
        .vector_store(store.clone())
        .build()?;

    indexer
        .index(
            Document::builder()
                .id("manual")
                .content("Para cancelar uma matrícula acesse o portal acadêmico.")
                .build()?,
        )
        .await?;

    let retriever = Retriever::new(embeddings, store);
    let pipeline = RagPipeline::builder()
        .retriever(retriever)
        .context_builder(Arc::new(SimpleContextBuilder::new(ContextConfig::default())))
        .build()?;

    let result = pipeline
        .run(RagRequest::new("como cancelar a matrícula?", 3))
        .await?;

    println!("{}", result.context);
    Ok(())
}
```

## Filtros de metadados

Metadados definidos no `Document` são propagados aos chunks e podem filtrar a
recuperação:

```rust,no_run
use gerax_rag::vector::{Filter, VectorFilter};
use gerax_rag::pipeline::RagRequest;

let request = RagRequest::new("como cancelar a matrícula?", 3)
    .with_filter(VectorFilter::and([Filter::eq("repository", "gerax")]));
```

## Exemplos executáveis

| Exemplo | Descrição |
| --- | --- |
| [`simple_rag`](examples/simple_rag.rs) | Indexação + pipeline RAG completos. |
| [`indexing`](examples/indexing.rs) | Demonstração isolada do `DocumentIndexer`. |
| [`semantic_search`](examples/semantic_search.rs) | Busca semântica direta via `VectorStore`. |
| [`markdown_rag`](examples/markdown_rag.rs) | RAG com `MarkdownChunker` (seções por heading). |

Para rodar um exemplo (substitua `simple_rag` pelo nome desejado):

```sh
cargo run -p gerax-rag --example simple_rag
```

## Testes

```sh
cargo test -p gerax-rag          # 80 testes unitários + doctest
cargo clippy -p gerax-rag --all-targets -- -D warnings
cargo fmt --package gerax-rag --check
```

## Expondo providers reais

Para conectar um provider real (ex.: Ollama, Qdrant), crie uma crate separada no
workspace implementando os traits e use o tipo concreto através de
`Arc<dyn Trait>`:

- `EmbeddingProvider` (embedding): `embed` / `embed_batch`.
- `VectorStore` (vetores): `upsert` / `get` / `delete` / `search`.
- `Reranker` (opcional): `rerank`.

O fluxo de setup do `DocumentIndexer` e `RagPipeline` permanece inalterado.