# Gerax AI Agent Skill — Implementar `gerax-rag`

## Objetivo

Implementar a crate `gerax-rag` como o módulo responsável pela arquitetura completa de **Retrieval-Augmented Generation (RAG)** do ecossistema Gerax.

A implementação deve ser:

- idiomática em Rust;
- assíncrona quando houver I/O;
- baseada em traits e abstrações;
- compatível com arquitetura Hexagonal;
- desacoplada de providers específicos;
- extensível para diferentes LLMs;
- extensível para diferentes bancos vetoriais;
- preparada para uso por agentes de IA;
- preparada para RAG simples e RAG avançado;
- organizada para futura integração com `gerax-ai`, `gerax-mcp` e agentes.

A crate não deve depender diretamente de:

- OpenAI;
- Ollama;
- Qdrant;
- PostgreSQL;
- pgvector;
- modelos específicos.

Essas integrações devem existir através de abstrações e adapters.

---

# 1. Visão arquitetural

A arquitetura desejada é:

```text
                    ┌───────────────────┐
                    │     Document      │
                    └─────────┬─────────┘
                              │
                              ▼
                    ┌───────────────────┐
                    │      Chunker      │
                    └─────────┬─────────┘
                              │
                              ▼
                       Vec<Chunk>
                              │
                              ▼
                    ┌───────────────────┐
                    │ EmbeddingProvider │
                    └─────────┬─────────┘
                              │
                              ▼
                       Embeddings
                              │
                              ▼
                    ┌───────────────────┐
                    │   VectorStore     │
                    └───────────────────┘
```

Consulta:

```text
User Query
    │
    ▼
EmbeddingProvider
    │
    ▼
Query Embedding
    │
    ▼
VectorStore.search()
    │
    ▼
Retrieved Chunks
    │
    ▼
Reranker (opcional)
    │
    ▼
Context Builder
    │
    ▼
Prompt
    │
    ▼
LLM / Agent
```

---

# 2. Responsabilidades da crate

A crate `gerax-rag` deve fornecer:

```text
gerax-rag
│
├── document
│
├── chunking
│
├── embedding
│
├── vector
│
├── indexing
│
├── retrieval
│
├── reranking
│
├── context
│
├── query
│
└── pipeline
```

A crate deve funcionar tanto:

```text
Document
   ↓
Indexação
```

quanto:

```text
Query
   ↓
Retrieval
   ↓
Context
```

---

# FASE 1 — Estrutura da crate

## Tarefa 1.1 — Criar a crate

Criar:

```text
gerax-rag/
├── Cargo.toml
└── src/
    ├── lib.rs
    │
    ├── document/
    ├── chunking/
    ├── embedding/
    ├── vector/
    ├── indexing/
    ├── retrieval/
    ├── reranking/
    ├── context/
    ├── query/
    └── pipeline/
```

Atualizar o workspace.

---

## Tarefa 1.2 — Definir dependências mínimas

Priorizar:

```toml
async-trait
thiserror
serde
serde_json
tokio
```

Não adicionar dependências específicas de providers.

Evitar dependências desnecessárias.

---

## Tarefa 1.3 — Criar `lib.rs`

Exportar módulos públicos:

```rust
pub mod document;
pub mod chunking;
pub mod embedding;
pub mod vector;
pub mod indexing;
pub mod retrieval;
pub mod reranking;
pub mod context;
pub mod query;
pub mod pipeline;
```

Reexportar os principais tipos.

O objetivo é permitir:

```rust
use gerax_rag::{
    Document,
    Chunk,
    Chunker,
    EmbeddingProvider,
    VectorStore,
    DocumentIndexer,
    Retriever,
};
```

---

# FASE 2 — Modelo de documentos

## Tarefa 2.1 — Implementar `Document`

Criar:

```rust
#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
}
```

---

## Tarefa 2.2 — Implementar `DocumentMetadata`

Inicialmente:

```rust
pub type DocumentMetadata =
    HashMap<String, serde_json::Value>;
```

A metadata deve permitir:

```text
source
file_name
language
author
created_at
section
repository
branch
url
```

Não criar um schema rígido.

---

## Tarefa 2.3 — Implementar builder

Permitir:

```rust
let document = Document::builder()
    .id("manual-alunos")
    .content(content)
    .metadata("source", "manual")
    .metadata("language", "pt-BR")
    .build()?;
```

Validar:

- id não vazio;
- content não vazio.

---

# FASE 3 — Chunking

## Tarefa 3.1 — Criar `Chunk`

```rust
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,

    pub document_id: String,

    pub index: usize,

    pub content: String,

    pub metadata: DocumentMetadata,
}
```

---

## Tarefa 3.2 — Criar `ChunkRequest`

```rust
pub struct ChunkRequest {
    pub document: Document,
}
```

A API deve permitir futura configuração por request.

---

## Tarefa 3.3 — Criar trait `Chunker`

```rust
pub trait Chunker: Send + Sync {
    fn chunk(
        &self,
        request: ChunkRequest,
    ) -> Result<Vec<Chunk>, ChunkError>;
}
```

O Chunker não deve conhecer:

- EmbeddingProvider;
- VectorStore;
- LLM.

---

# FASE 4 — Estratégias de Chunking

## Tarefa 4.1 — Implementar `FixedSizeChunker`

Configuração:

```rust
pub struct FixedSizeChunker {
    pub chunk_size: usize,
    pub overlap: usize,
}
```

Validar:

```text
chunk_size > 0

overlap < chunk_size
```

Implementar:

```text
Document
   ↓
FixedSizeChunker
   ↓
Chunk 0
Chunk 1
Chunk 2
```

---

## Tarefa 4.2 — Implementar `RecursiveChunker`

Separadores configuráveis:

```rust
[
    "\n\n",
    "\n",
    ". ",
    " ",
]
```

Algoritmo:

```text
tentar maior unidade semântica
        ↓
cabe no chunk?
    │        │
   sim      não
    │        │
    ▼        ▼
 adicionar   dividir usando
              próximo separador
```

Preservar:

- parágrafos;
- frases;
- contexto.

---

## Tarefa 4.3 — Implementar `MarkdownChunker`

Deve reconhecer:

```markdown
# Heading 1
## Heading 2
### Heading 3
```

Preservar a hierarquia.

Exemplo:

```text
Document

# Alunos

## Matrícula

Conteúdo...
```

Chunk:

```text
# Alunos

## Matrícula

Conteúdo...
```

Adicionar metadata:

```text
heading
heading_path
```

Exemplo:

```text
Alunos > Matrícula
```

---

## Tarefa 4.4 — Preparar trait para Chunkers especializados

Permitir futuras implementações:

```text
RustCodeChunker
OpenApiChunker
JsonChunker
HtmlChunker
PdfChunker
```

Não implementar todos inicialmente.

Criar interfaces e extensão sem quebrar a API.

---

# FASE 5 — Embeddings

## Tarefa 5.1 — Criar `Embedding`

```rust
#[derive(Debug, Clone)]
pub struct Embedding {
    pub vector: Vec<f32>,
}
```

Implementar:

```rust
impl Embedding {
    pub fn dimensions(&self) -> usize {
        self.vector.len()
    }
}
```

---

## Tarefa 5.2 — Criar trait `EmbeddingProvider`

```rust
#[async_trait]
pub trait EmbeddingProvider:
    Send + Sync
{
    fn model(&self) -> &str;

    fn dimensions(&self) -> Option<usize>;

    async fn embed(
        &self,
        text: &str,
    ) -> Result<Embedding, EmbeddingError>;

    async fn embed_batch(
        &self,
        texts: &[String],
    ) -> Result<
        Vec<Embedding>,
        EmbeddingError,
    >;
}
```

---

## Tarefa 5.3 — Criar `MockEmbeddingProvider`

Implementação determinística.

Objetivos:

- testes;
- exemplos;
- CI;
- não depender de APIs externas.

O mock não precisa gerar embeddings semanticamente reais.

---

# FASE 6 — Armazenamento vetorial

## Tarefa 6.1 — Criar `VectorDocument`

```rust
#[derive(Debug, Clone)]
pub struct VectorDocument {
    pub id: String,

    pub vector: Embedding,

    pub content: String,

    pub metadata: DocumentMetadata,
}
```

---

## Tarefa 6.2 — Criar `VectorSearchResult`

```rust
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub document: VectorDocument,

    pub score: f32,
}
```

---

## Tarefa 6.3 — Criar `VectorStore`

```rust
#[async_trait]
pub trait VectorStore:
    Send + Sync
{
    async fn upsert(
        &self,
        document: VectorDocument,
    ) -> Result<(), VectorStoreError>;

    async fn get(
        &self,
        id: &str,
    ) -> Result<
        Option<VectorDocument>,
        VectorStoreError,
    >;

    async fn delete(
        &self,
        id: &str,
    ) -> Result<(), VectorStoreError>;

    async fn search(
        &self,
        request: VectorSearchRequest,
    ) -> Result<
        Vec<VectorSearchResult>,
        VectorStoreError,
    >;
}
```

---

## Tarefa 6.4 — Criar `VectorSearchRequest`

```rust
pub struct VectorSearchRequest {
    pub vector: Embedding,

    pub limit: usize,

    pub min_score: Option<f32>,

    pub filter: Option<VectorFilter>,
}
```

Preparar a API para filtros futuros.

---

# FASE 7 — Similaridade

## Tarefa 7.1 — Implementar Cosine Similarity

Criar:

```rust
pub fn cosine_similarity(
    a: &[f32],
    b: &[f32],
) -> Result<f32, SimilarityError>;
```

Validar:

```text
vetores vazios
dimensões diferentes
vetor de norma zero
```

Não fazer `unwrap()`.

---

## Tarefa 7.2 — Criar abstração de Similaridade

Preparar:

```rust
pub trait SimilarityMetric:
    Send + Sync
{
    fn similarity(
        &self,
        a: &[f32],
        b: &[f32],
    ) -> Result<f32, SimilarityError>;
}
```

Implementar inicialmente:

```text
CosineSimilarity
```

Preparar futuras:

```text
DotProductSimilarity
EuclideanDistance
```

---

# FASE 8 — `InMemoryVectorStore`

## Tarefa 8.1 — Implementar armazenamento

Usar:

```rust
Arc<RwLock<HashMap<String, VectorDocument>>>
```

ou estrutura equivalente idiomática.

---

## Tarefa 8.2 — Implementar busca

Fluxo:

```text
Query Vector
    │
    ▼
Para cada documento
    │
    ▼
SimilarityMetric
    │
    ▼
Score
    │
    ▼
Ordenar descrescente
    │
    ▼
Aplicar min_score
    │
    ▼
Top K
```

---

## Tarefa 8.3 — Garantir thread safety

O store deve funcionar em ambiente async concorrente.

Adicionar testes de:

- inserção;
- atualização;
- busca;
- exclusão;
- concorrência básica.

---

# FASE 9 — Indexação

## Tarefa 9.1 — Criar `DocumentIndexer`

```rust
pub struct DocumentIndexer {
    chunker: Arc<dyn Chunker>,

    embedding_provider:
        Arc<dyn EmbeddingProvider>,

    vector_store:
        Arc<dyn VectorStore>,
}
```

---

## Tarefa 9.2 — Implementar fluxo de indexação

```text
Document
   │
   ▼
Chunker
   │
   ▼
Vec<Chunk>
   │
   ▼
EmbeddingProvider.embed_batch()
   │
   ▼
Vec<Embedding>
   │
   ▼
VectorDocument
   │
   ▼
VectorStore.upsert()
```

Priorizar batch embeddings.

Não gerar embeddings individualmente quando o provider suporta batch.

---

## Tarefa 9.3 — Criar resultado da indexação

```rust
pub struct IndexingResult {
    pub document_id: String,

    pub chunks_created: usize,

    pub chunks_indexed: usize,
}
```

Preparar espaço para:

```text
duration
tokens
model
errors
```

---

# FASE 10 — Retrieval

## Tarefa 10.1 — Criar `Retriever`

```rust
pub struct Retriever {
    embedding_provider:
        Arc<dyn EmbeddingProvider>,

    vector_store:
        Arc<dyn VectorStore>,
}
```

---

## Tarefa 10.2 — Criar `RetrievalRequest`

```rust
pub struct RetrievalRequest {
    pub query: String,

    pub limit: usize,

    pub min_score: Option<f32>,
}
```

---

## Tarefa 10.3 — Criar `RetrievalResult`

```rust
pub struct RetrievalResult {
    pub query: String,

    pub results:
        Vec<VectorSearchResult>,
}
```

---

## Tarefa 10.4 — Implementar fluxo

```text
Query
   │
   ▼
EmbeddingProvider
   │
   ▼
Query Embedding
   │
   ▼
VectorStore.search()
   │
   ▼
Resultados
```

---

# FASE 11 — Reranking

## Tarefa 11.1 — Criar trait `Reranker`

```rust
#[async_trait]
pub trait Reranker:
    Send + Sync
{
    async fn rerank(
        &self,
        query: &str,
        documents: Vec<VectorSearchResult>,
    ) -> Result<
        Vec<VectorSearchResult>,
        RerankerError,
    >;
}
```

---

## Tarefa 11.2 — Criar `NoopReranker`

Implementação padrão:

```text
entrada
  ↓
saída sem alteração
```

Isso permite:

```text
Retriever
   ↓
Optional Reranker
```

sem exigir provider externo.

---

## Tarefa 11.3 — Preparar integração futura

Futuras implementações:

```text
CrossEncoderReranker
LLMReranker
CohereReranker
LocalModelReranker
```

Não implementar providers específicos nesta fase.

---

# FASE 12 — Context Builder

## Tarefa 12.1 — Criar `ContextBuilder`

Responsabilidade:

```text
Retrieved Documents
       ↓
Context
```

Trait:

```rust
pub trait ContextBuilder:
    Send + Sync
{
    fn build(
        &self,
        results: &[VectorSearchResult],
    ) -> Result<String, ContextError>;
}
```

---

## Tarefa 12.2 — Implementar `SimpleContextBuilder`

Formato:

```text
[Source: document-id]

Conteúdo...

---

[Source: document-id]

Conteúdo...
```

---

## Tarefa 12.3 — Adicionar limite

Configuração:

```rust
pub struct ContextConfig {
    pub max_characters: usize,
}
```

O builder deve parar antes de ultrapassar o limite.

Futuramente o limite deve poder ser baseado em tokens.

---

# FASE 13 — Pipeline RAG

## Tarefa 13.1 — Criar `RagPipeline`

```rust
pub struct RagPipeline {
    retriever: Retriever,

    reranker:
        Option<Arc<dyn Reranker>>,

    context_builder:
        Arc<dyn ContextBuilder>,
}
```

---

## Tarefa 13.2 — Criar `RagRequest`

```rust
pub struct RagRequest {
    pub query: String,

    pub retrieval_limit: usize,

    pub min_score: Option<f32>,
}
```

---

## Tarefa 13.3 — Criar `RagResult`

```rust
pub struct RagResult {
    pub query: String,

    pub documents:
        Vec<VectorSearchResult>,

    pub context: String,
}
```

---

## Tarefa 13.4 — Implementar pipeline

```text
Query
   │
   ▼
Retriever
   │
   ▼
Documents
   │
   ▼
Reranker?
   │
   ▼
ContextBuilder
   │
   ▼
RagResult
```

Importante:

`gerax-rag` não deve obrigatoriamente chamar o LLM.

O resultado deve ser utilizável por:

```text
gerax-ai
Agent
MCP
Application
Chat System
```

---

# FASE 14 — Integração com LLM

Esta fase deve ser opcional.

Criar uma abstração que permita:

```text
RAG Context
    +
User Prompt
    ↓
LLM
```

Mas não acoplar `gerax-rag` a um provider específico.

Preferir integração através de uma trait externa do ecossistema Gerax.

Exemplo conceitual:

```rust
pub trait RagGenerator {
    async fn generate(
        &self,
        query: &str,
        context: &str,
    ) -> Result<String, GenerationError>;
}
```

Se `gerax-ai` já possuir abstração equivalente, reutilizá-la.

Não duplicar abstrações existentes.

---

# FASE 15 — Metadata Filtering

Preparar filtros.

Exemplo:

```text
repository = gerax
language = rust
branch = main
```

API conceitual:

```rust
VectorFilter::and([
    Filter::eq("repository", "gerax"),
    Filter::eq("language", "rust"),
])
```

A implementação inicial pode ser simples.

A API deve ser transportável para:

```text
InMemory
Qdrant
PgVector
LanceDB
```

Evitar filtros específicos de banco.

---

# FASE 16 — Namespaces e Collections

Preparar isolamento lógico.

Exemplo:

```text
gerax-docs
escola-cqrs
project-a
project-b
```

Adicionar:

```rust
pub struct CollectionName(String);
```

Permitir:

```rust
VectorDocument {
    collection: CollectionName::new("gerax-docs"),
    ...
}
```

A API deve permitir que um RAG tenha múltiplas bases de conhecimento.

---

# FASE 17 — Observabilidade

Preparar eventos ou hooks.

Exemplo:

```text
DocumentIndexed
ChunkCreated
EmbeddingGenerated
VectorStored
RetrievalPerformed
RerankingPerformed
```

Não adicionar framework pesado de observabilidade.

Preferir integração com o mecanismo de logging/tracing do workspace.

---

# FASE 18 — Erros

Criar módulos específicos:

```text
DocumentError
ChunkError
EmbeddingError
VectorStoreError
SimilarityError
IndexError
RetrievalError
RerankerError
ContextError
RagError
```

Evitar:

```rust
Box<dyn Error>
```

na API pública.

Preferir erros tipados.

Permitir conversões idiomáticas:

```rust
#[from]
```

quando fizer sentido.

---

# FASE 19 — Builders

Criar builders para componentes complexos.

Exemplo:

```rust
let pipeline = RagPipeline::builder()
    .retriever(retriever)
    .reranker(reranker)
    .context_builder(context_builder)
    .build()?;
```

E:

```rust
let indexer = DocumentIndexer::builder()
    .chunker(chunker)
    .embedding_provider(provider)
    .vector_store(store)
    .build()?;
```

Validar dependências obrigatórias.

---

# FASE 20 — Exemplos

Criar:

```text
examples/
├── simple_rag.rs
├── indexing.rs
├── semantic_search.rs
└── markdown_rag.rs
```

---

## Exemplo obrigatório: `simple_rag.rs`

Demonstrar:

```text
Document
   ↓
Chunk
   ↓
Embedding
   ↓
InMemoryVectorStore
   ↓
Query
   ↓
Retrieval
   ↓
Context
```

Sem API externa.

Deve funcionar usando:

```text
MockEmbeddingProvider
```

---

# FASE 21 — Testes

Criar testes para:

```text
Document
Chunker
FixedSizeChunker
RecursiveChunker
MarkdownChunker
CosineSimilarity
InMemoryVectorStore
DocumentIndexer
Retriever
ContextBuilder
RagPipeline
```

---

## Testes obrigatórios

### Chunking

```text
documento vazio
documento pequeno
documento grande
overlap
fronteiras
unicode
português
```

---

### Similaridade

```text
vetores iguais
vetores opostos
vetores ortogonais
dimensões diferentes
vetor vazio
vetor zero
```

---

### Vector Store

```text
upsert
update
get
delete
search
limit
min_score
```

---

### Pipeline

```text
indexar
consultar
recuperar
rerank noop
montar contexto
```

---

# FASE 22 — Qualidade de código

Antes de considerar a implementação concluída:

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Corrigir:

- warnings;
- dead code;
- clones desnecessários;
- allocations desnecessárias;
- `unwrap()` em código de produção;
- APIs públicas inconsistentes.

---

# FASE 23 — Integração com arquitetura Gerax

A crate deve respeitar:

```text
Application
     │
     ▼
Ports
     │
     ▼
Traits
     │
     ▼
Adapters
```

Exemplo:

```text
                gerax-rag
                    │
                    │ Ports
                    ▼

        ┌───────────┼───────────┐
        │           │           │
        ▼           ▼           ▼

EmbeddingProvider VectorStore Reranker

        │           │           │

        ▼           ▼           ▼

     Ollama       Qdrant    CrossEncoder
     OpenAI       PgVector
     Local        LanceDB
```

`gerax-rag` deve conter principalmente:

- domínio;
- abstrações;
- orquestração;
- implementação in-memory;
- implementações genéricas.

Providers específicos devem preferencialmente existir em crates separadas.

Exemplo futuro:

```text
gerax-rag
gerax-rag-ollama
gerax-rag-openai
gerax-rag-qdrant
gerax-rag-pgvector
```

Ou, caso o projeto prefira:

```text
gerax-ai
gerax-ai-ollama
gerax-ai-openai
gerax-qdrant
gerax-postgres
```

O agente deve verificar a arquitetura existente do workspace antes de criar crates redundantes.

---

# Critérios de aceitação

A implementação estará concluída quando for possível executar conceitualmente:

```rust
let document = Document::builder()
    .id("manual")
    .content(
        "Para cancelar uma matrícula..."
    )
    .build()?;

indexer.index(document).await?;

let result = pipeline
    .run(
        RagRequest {
            query:
                "Como desfazer a inscrição?"
                    .into(),

            retrieval_limit: 5,

            min_score: Some(0.5),
        }
    )
    .await?;

println!("{}", result.context);
```

E o fluxo interno for:

```text
INDEXAÇÃO

Document
   │
   ▼
Chunker
   │
   ▼
Chunks
   │
   ▼
EmbeddingProvider
   │
   ▼
Embeddings
   │
   ▼
VectorStore
```

E:

```text
CONSULTA

Query
   │
   ▼
EmbeddingProvider
   │
   ▼
VectorStore.search
   │
   ▼
Retrieved Chunks
   │
   ▼
Reranker
   │
   ▼
ContextBuilder
   │
   ▼
RagResult
```

---

# Ordem obrigatória de execução pelo agente

O agente deve implementar nesta ordem:

```text
FASE 1
Estrutura
    ↓
FASE 2
Document
    ↓
FASE 3
Chunk + Chunker
    ↓
FASE 4
FixedSizeChunker
    ↓
FASE 5
EmbeddingProvider + Mock
    ↓
FASE 6
Similarity
    ↓
FASE 7
VectorStore
    ↓
FASE 8
InMemoryVectorStore
    ↓
FASE 9
DocumentIndexer
    ↓
FASE 10
Retriever
    ↓
FASE 11
ContextBuilder
    ↓
FASE 12
RagPipeline
    ↓
FASE 13
Testes end-to-end
    ↓
FASE 14
RecursiveChunker
    ↓
FASE 15
MarkdownChunker
    ↓
FASE 16
Reranker
    ↓
FASE 17
Metadata filters
    ↓
FASE 18
Observabilidade
```

---

# Regras para o agente

1. Não implementar todas as fases em um único passo.

2. Executar uma fase por vez.

3. Antes de cada fase:
   - analisar o código existente;
   - verificar APIs já existentes no workspace;
   - evitar duplicação.

4. Após cada fase:
   - executar `cargo fmt`;
   - executar `cargo check`;
   - executar os testes relevantes.

5. Não usar `unwrap()` ou `expect()` em código de produção.

6. Não acoplar o core do RAG a providers externos.

7. Não criar abstrações duplicadas se `gerax-ai` ou outra crate do workspace já fornecer uma adequada.

8. Preferir tipos explícitos na API pública.

9. Manter ownership e clonagem idiomáticos.

10. Usar `Arc<dyn Trait>` apenas onde o polimorfismo dinâmico for realmente necessário.

11. Implementar uma versão funcional mínima antes de adicionar recursos avançados.

12. Manter a API preparada para evolução sem introduzir complexidade prematura.

---

# Resultado esperado

Ao final, `gerax-rag` deve fornecer uma base completa para:

```text
RAG
Semantic Search
Knowledge Base
Documentation Search
Agent Context
Code Search
MCP Knowledge Retrieval
Semantic Tool Discovery
```

E deve permitir a composição:

```text
                  ┌─────────────┐
                  │ gerax-ai    │
                  └──────┬──────┘
                         │
                         ▼
                  ┌─────────────┐
                  │ gerax-rag   │
                  └──────┬──────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼

      Embeddings     Vector Store     Reranker
          │              │              │
          ▼              ▼              ▼

       Providers       Databases       Models
```

O objetivo final é que `gerax-rag` seja o **núcleo de recuperação semântica e construção de contexto** do ecossistema Gerax, enquanto as implementações específicas de modelos, bancos vetoriais e serviços externos permaneçam desacopladas através de ports e adapters.