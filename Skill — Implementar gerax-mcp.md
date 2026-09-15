# Skill: Implementar `gerax-mcp`

## Objetivo

Implementar a crate `gerax-mcp` no workspace Gerax, fornecendo uma implementação Rust idiomática do **Model Context Protocol (MCP)** para permitir que aplicações Gerax exponham:

- Tools
- Resources
- Prompts
- JSON-RPC
- MCP Server
- MCP Client, quando aplicável
- descoberta de capacidades
- execução de ferramentas
- leitura de recursos
- obtenção de prompts
- integração com `gerax-cqrs`
- integração futura com `gerax-rag`
- integração com agentes de IA

A implementação deve seguir arquitetura modular, tipada e extensível, evitando acoplamento desnecessário com qualquer framework HTTP específico.

---

# 1. Contexto arquitetural

O workspace Gerax possui arquitetura modular baseada em crates.

A nova crate deverá ser:

```text
gerax-mcp/
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── jsonrpc.rs
│   │   ├── initialize.rs
│   │   ├── tools.rs
│   │   ├── resources.rs
│   │   ├── prompts.rs
│   │   └── notifications.rs
│   ├── server/
│   │   ├── mod.rs
│   │   ├── server.rs
│   │   ├── capabilities.rs
│   │   └── registry.rs
│   ├── tool/
│   │   ├── mod.rs
│   │   ├── tool.rs
│   │   ├── handler.rs
│   │   └── registry.rs
│   ├── resource/
│   │   ├── mod.rs
│   │   ├── resource.rs
│   │   └── registry.rs
│   ├── prompt/
│   │   ├── mod.rs
│   │   ├── prompt.rs
│   │   └── registry.rs
│   └── transport/
│       ├── mod.rs
│       ├── stdio.rs
│       └── http.rs
└── Cargo.toml
```

A implementação deve separar:

```text
MCP Protocol
     │
     ▼
JSON-RPC
     │
     ▼
MCP Server
     │
 ┌───┼─────────────┐
 ▼   ▼             ▼
Tools Resources   Prompts
 │
 ▼
Gerax application
 │
 ├── CQRS
 ├── RAG
 ├── Domain Services
 └── External Services
```

---

# 2. Princípios obrigatórios

O agente deverá seguir estas regras:

1. Rust idiomático.
2. `async/await`.
3. Tipagem forte.
4. Erros explícitos.
5. Evitar `Box<dyn Any>` quando uma solução tipada for possível.
6. Separar protocolo, domínio MCP e transporte.
7. Não acoplar o núcleo MCP a Actix, Axum ou Poem.
8. Permitir múltiplos transports.
9. Permitir registro dinâmico de Tools.
10. Permitir registro dinâmico de Resources.
11. Permitir registro dinâmico de Prompts.
12. APIs públicas pequenas e previsíveis.
13. Testes unitários e de integração.
14. Documentação Rustdoc para APIs públicas.
15. Não introduzir dependências sem justificar sua necessidade.
16. Preservar compatibilidade com as demais crates Gerax.

---

# FASE 1 — Criar a crate

## Tarefa 1.1 — Criar `gerax-mcp`

Adicionar ao workspace:

```toml
members = [
    ...
    "gerax-mcp",
]
```

Criar:

```text
gerax-mcp/
├── Cargo.toml
└── src/
    └── lib.rs
```

Configurar:

```toml
edition = "2024"
rust-version = "1.96.1"
license = "MIT OR Apache-2.0"
```

---

## Tarefa 1.2 — Dependências

Selecionar somente as dependências necessárias.

Possíveis dependências:

```text
serde
serde_json
thiserror
async-trait
tokio
```

Para transporte HTTP, não adicionar Actix/Axum diretamente ao núcleo.

Se necessário, criar integrações separadas:

```text
gerax-mcp
gerax-mcp-http
```

ou adapters opcionais.

---

## Critério de conclusão

Executar:

```bash
cargo check -p gerax-mcp
cargo test -p gerax-mcp
```

---

# FASE 2 — Implementar JSON-RPC

O MCP utiliza JSON-RPC.

Criar tipos fundamentais:

```rust
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    pub params: Option<serde_json::Value>,
}
```

```rust
pub enum JsonRpcId {
    Number(i64),
    String(String),
}
```

Criar resposta:

```rust
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}
```

Erro:

```rust
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
```

Implementar:

```text
request
response
error
notification
id
params
result
```

Testar serialização e desserialização.

---

# FASE 3 — Modelo MCP

Criar os tipos fundamentais do protocolo.

## Initialize

Implementar:

```text
initialize
notifications/initialized
```

Criar:

```rust
pub struct InitializeRequest {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ImplementationInfo,
}
```

```rust
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ImplementationInfo,
}
```

---

# FASE 4 — Server

Criar:

```rust
pub struct McpServer {
    ...
}
```

API desejada:

```rust
let server = McpServer::builder()
    .name("escola-cqrs")
    .version("0.1.0")
    .build();
```

O server deverá possuir registries independentes:

```text
ToolRegistry
ResourceRegistry
PromptRegistry
```

---

# FASE 5 — Tool

Implementar abstração:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> Option<&str>;

    fn input_schema(&self) -> serde_json::Value;

    async fn call(
        &self,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ToolError>;
}
```

Criar:

```rust
ToolRegistry
```

API desejada:

```rust
registry.register(MyTool::new());
```

Consulta:

```rust
registry.get("create_aluno");
```

Listagem:

```rust
registry.list();
```

Execução:

```rust
registry.call(
    "create_aluno",
    arguments,
).await?;
```

---

# FASE 6 — Tool baseada em função

Fornecer uma API conveniente.

Exemplo:

```rust
let tool = tool(
    "calculate_average",
    "Calculates the average of grades",
    schema,
    |args| async move {
        ...
    },
);
```

Ou uma abstração tipada:

```rust
pub trait ToolHandler {
    type Input;
    type Output;

    async fn call(
        &self,
        input: Self::Input,
    ) -> Result<Self::Output, ToolError>;
}
```

Usar `serde`/JSON Schema para converter:

```text
Input Rust
    │
    ▼
JSON Schema
    │
    ▼
MCP Tool
```

---

# FASE 7 — Integração com `gerax-cqrs`

Essa é uma integração importante.

Permitir transformar Commands em MCP Tools.

Arquitetura:

```text
LLM / Agent
     │
     ▼
MCP Tool
     │
     ▼
CommandBus
     │
     ▼
CommandHandler
     │
     ▼
Domain
```

Exemplo conceitual:

```rust
CreateAluno
    ↓
CreateAlunoHandler
    ↓
CommandBus
    ↓
MCP Tool
```

Criar adapter:

```rust
pub struct CommandTool<C, B> {
    ...
}
```

Possibilitar algo como:

```rust
server.register_command::<CreateAluno>();
```

ou:

```rust
server.tool(
    CommandTool::new(command_bus)
);
```

O agente deverá analisar a API existente de `gerax-cqrs` antes de implementar essa integração.

Não duplicar `CommandBus`, `CommandHandler` ou `CommandMetadata`.

---

# FASE 8 — Resource

Implementar MCP Resources.

Criar:

```rust
pub trait Resource: Send + Sync {
    fn uri(&self) -> &str;

    fn name(&self) -> &str;

    fn description(&self) -> Option<&str>;

    async fn read(
        &self,
    ) -> Result<ResourceContents, ResourceError>;
}
```

Criar:

```rust
ResourceRegistry
```

API:

```rust
server.resources()
    .register(MyResource::new());
```

Implementar operações equivalentes a:

```text
resources/list
resources/read
```

---

# FASE 9 — Prompt

Implementar MCP Prompts.

Criar:

```rust
pub trait Prompt: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> Option<&str>;

    async fn get(
        &self,
        arguments: serde_json::Value,
    ) -> Result<GetPromptResult, PromptError>;
}
```

Criar:

```rust
PromptRegistry
```

Implementar:

```text
prompts/list
prompts/get
```

---

# FASE 10 — Capabilities

Implementar corretamente as capabilities.

Criar:

```rust
pub struct ServerCapabilities {
    ...
}
```

Permitir declarar:

```text
tools
resources
prompts
logging
```

A resposta de `initialize` deve refletir somente os recursos efetivamente registrados/habilitados.

---

# FASE 11 — Notifications

Implementar suporte a notifications.

Especialmente:

```text
notifications/initialized
```

O design deve permitir futuramente:

```text
notifications/tools/list_changed
notifications/resources/list_changed
notifications/prompts/list_changed
```

Não implementar funcionalidades futuras apenas como placeholders sem necessidade; preparar uma arquitetura extensível.

---

# FASE 12 — Dispatcher

Criar o componente responsável por transformar JSON-RPC em operações MCP.

Exemplo:

```rust
pub struct McpDispatcher {
    server: Arc<McpServer>,
}
```

Fluxo:

```text
JsonRpcRequest
      │
      ▼
McpDispatcher
      │
      ├── initialize
      ├── tools/list
      ├── tools/call
      ├── resources/list
      ├── resources/read
      ├── prompts/list
      └── prompts/get
```

O dispatcher não deve conhecer detalhes de HTTP ou STDIO.

---

# FASE 13 — Transport

Criar abstração:

```rust
#[async_trait]
pub trait Transport {
    async fn receive(&mut self)
        -> Result<JsonRpcRequest, TransportError>;

    async fn send(
        &mut self,
        response: JsonRpcResponse,
    ) -> Result<(), TransportError>;
}
```

Implementar inicialmente:

```text
StdioTransport
```

Fluxo:

```text
stdin
  ↓
JSON-RPC
  ↓
McpDispatcher
  ↓
JSON-RPC
  ↓
stdout
```

---

# FASE 14 — HTTP

Preparar integração HTTP sem contaminar o núcleo.

Criar uma camada específica, por exemplo:

```text
gerax-mcp-http
```

ou feature:

```toml
[features]
http = [...]
```

Permitir integração com:

```text
gerax-http
gerax-axum
gerax-actix
gerax-poem
gerax-salvo
```

A lógica MCP deve permanecer em:

```text
gerax-mcp
```

---

# FASE 15 — MCP Server para Gerax

Criar uma API de alto nível:

```rust
let server = McpServer::builder()
    .name("escola-cqrs")
    .version("0.1.0")
    .tool(CreateAlunoTool::new(...))
    .tool(MatricularAlunoTool::new(...))
    .resource(AlunoResource::new(...))
    .prompt(ProfessorPrompt::new(...))
    .build();
```

Executar:

```rust
server.run(StdioTransport::new()).await?;
```

---

# FASE 16 — Macros

Somente depois da API manual estar estável, criar macros.

Possível API:

```rust
#[derive(McpTool)]
#[mcp(
    name = "create_aluno",
    description = "Creates an aluno"
)]
pub struct CreateAluno {
    pub nome: String,
    pub email: String,
}
```

Ou:

```rust
#[mcp_tool]
async fn create_aluno(
    input: CreateAluno,
) -> Result<Aluno, Error> {
    ...
}
```

A macro deverá gerar:

```text
Tool metadata
JSON Schema
Tool handler
registration information
```

Não duplicar lógica do runtime.

---

# FASE 17 — Auto-registration

Investigar uma API semelhante à utilizada no `gerax-cqrs`.

Objetivo:

```rust
server.register(CreateAlunoTool::new(...));
```

ou futuramente:

```rust
server.register_all::<McpTools>();
```

As informações de registro devem vir do próprio tipo sempre que possível.

Evitar reflection runtime desnecessária.

---

# FASE 18 — Integração com `gerax-rag`

Preparar integração futura:

```text
MCP Tool
    │
    ▼
RAG Service
    │
    ├── Chunker
    ├── EmbeddingProvider
    └── VectorStore
```

Exemplos:

```text
rag_search
rag_get_document
rag_list_documents
```

Não implementar RAG dentro de `gerax-mcp`.

Criar apenas adapters/interfaces quando necessário.

---

# FASE 19 — Segurança

Implementar pontos de extensão para:

```text
authentication
authorization
tool permissions
resource permissions
rate limiting
```

Não colocar autenticação específica dentro do core.

Exemplo:

```rust
pub trait McpAuthorizer {
    async fn authorize(
        &self,
        context: &McpContext,
        operation: &McpOperation,
    ) -> Result<(), AuthorizationError>;
}
```

Fluxo:

```text
Request
   ↓
Authentication
   ↓
Authorization
   ↓
Dispatcher
   ↓
Tool
```

---

# FASE 20 — Context

Criar contexto MCP:

```rust
pub struct McpContext {
    ...
}
```

O contexto poderá futuramente conter:

```text
request metadata
client information
authentication
authorization
tracing
state
extensions
```

Evitar colocar dependências concretas de HTTP.

---

# FASE 21 — Error Handling

Criar erros separados:

```text
McpError
JsonRpcError
ToolError
ResourceError
PromptError
TransportError
AuthorizationError
```

Permitir conversões:

```rust
From<T> for McpError
```

Não utilizar:

```rust
Box<dyn Error>
```

como erro público principal.

---

# FASE 22 — Testes

Criar testes unitários para:

```text
JSON-RPC serialization
JSON-RPC deserialization
request IDs
errors
initialize
capabilities
tools/list
tools/call
resources/list
resources/read
prompts/list
prompts/get
notifications
registry
dispatcher
```

Criar testes de integração:

```text
Client
   ↓
Transport
   ↓
McpServer
   ↓
Tool
```

Exemplo:

```rust
#[tokio::test]
async fn should_call_create_aluno_tool() {
    ...
}
```

---

# FASE 23 — Teste completo com STDIO

Criar um exemplo:

```text
examples/server.rs
```

O exemplo deve iniciar um MCP server Gerax e expor pelo menos:

```text
1 Tool
1 Resource
1 Prompt
```

Executar:

```bash
cargo run -p gerax-mcp --example server
```

Validar comunicação JSON-RPC real via STDIO.

---

# FASE 24 — Exemplo com CQRS

Criar:

```text
examples/cqrs.rs
```

Arquitetura:

```text
MCP
 │
 ▼
CreateAlunoTool
 │
 ▼
CommandBus
 │
 ▼
CreateAlunoHandler
 │
 ▼
Repository
```

O exemplo deverá demonstrar que MCP é apenas uma interface de entrada e não contém regra de negócio.

---

# FASE 25 — Documentação

Criar:

```text
README.md
```

Documentar:

1. O que é `gerax-mcp`.
2. Arquitetura.
3. Instalação.
4. Criando um Tool.
5. Criando um Resource.
6. Criando um Prompt.
7. Criando um MCP Server.
8. STDIO.
9. HTTP.
10. Integração com CQRS.
11. Segurança.
12. Exemplos.

Adicionar Rustdoc nas APIs públicas.

---

# FASE 26 — Quality Gate

Antes de considerar a implementação concluída:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Verificar também:

```bash
cargo doc --workspace --no-deps
```

Não finalizar a tarefa se houver warnings introduzidos pela nova crate.

---

# FASE 27 — Revisão arquitetural

O agente deverá revisar a implementação procurando:

### Acoplamento

Verificar se:

```text
gerax-mcp
```

depende desnecessariamente de:

```text
gerax-actix
gerax-axum
gerax-poem
gerax-salvo
```

### Duplicação

Verificar se foram duplicados conceitos existentes em:

```text
gerax-core
gerax-http
gerax-cqrs
gerax-ai
```

### API pública

Verificar se a API é simples para o usuário.

Preferir:

```rust
server.tool(...)
```

a APIs excessivamente complexas.

---

# Resultado arquitetural esperado

Ao final, o ecossistema deverá permitir:

```text
                    ┌─────────────────────┐
                    │    AI Agent / LLM   │
                    └──────────┬──────────┘
                               │
                              MCP
                               │
                    ┌──────────▼──────────┐
                    │     gerax-mcp       │
                    │                     │
                    │  JSON-RPC           │
                    │  Dispatcher         │
                    │  Server             │
                    │  Registry           │
                    └──────────┬──────────┘
                               │
             ┌─────────────────┼─────────────────┐
             │                 │                 │
             ▼                 ▼                 ▼
          Tools            Resources         Prompts
             │
             ▼
       ┌───────────────┐
       │ Gerax Domain  │
       └───────┬───────┘
               │
       ┌───────┴────────┐
       │                │
       ▼                ▼
   gerax-cqrs       gerax-rag
       │                │
       ▼                ▼
 CommandBus        VectorStore
       │
       ▼
   Application
       │
       ▼
    Domain
```

---

# Regras para o agente

O agente deve:

1. Trabalhar em fases.
2. Não implementar todas as fases de uma única vez.
3. Ao terminar cada fase, executar os testes correspondentes.
4. Corrigir os erros antes de continuar.
5. Inspecionar as crates existentes antes de criar abstrações duplicadas.
6. Reutilizar tipos e traits existentes no Gerax.
7. Manter `gerax-mcp` independente de framework HTTP.
8. Criar adapters separados quando necessário.
9. Não adicionar macros antes da API runtime estar estável.
10. Não implementar funcionalidades MCP não necessárias apenas para "completar" a crate.
11. Manter compatibilidade com Rust 1.96.1.
12. Documentar decisões arquiteturais importantes.
13. Criar testes para cada feature pública.
14. Não alterar outras crates sem necessidade.
15. Ao detectar conflito com a arquitetura atual do Gerax, parar e documentar o conflito antes de fazer uma mudança estrutural.

---

# Ordem obrigatória de execução

```text
FASE 1  → Crate
FASE 2  → JSON-RPC
FASE 3  → MCP Protocol
FASE 4  → Server
FASE 5  → Tools
FASE 6  → Tool Handler
FASE 7  → CQRS
FASE 8  → Resources
FASE 9  → Prompts
FASE 10 → Capabilities
FASE 11 → Notifications
FASE 12 → Dispatcher
FASE 13 → Transport
FASE 14 → HTTP
FASE 15 → Gerax Server API
FASE 16 → Macros
FASE 17 → Auto-registration
FASE 18 → RAG
FASE 19 → Security
FASE 20 → Context
FASE 21 → Errors
FASE 22 → Tests
FASE 23 → STDIO example
FASE 24 → CQRS example
FASE 25 → Documentation
FASE 26 → Quality Gate
FASE 27 → Architectural Review
```

# Definition of Done

A implementação será considerada concluída quando:

- `gerax-mcp` compilar;
- todos os testes passarem;
- JSON-RPC estiver funcionando;
- `initialize` estiver implementado;
- Tools estiverem funcionando;
- Resources estiverem funcionando;
- Prompts estiverem funcionando;
- capabilities estiverem funcionando;
- STDIO estiver funcionando;
- dispatcher estiver funcionando;
- integração com `gerax-cqrs` estiver demonstrada;
- exemplos estiverem funcionando;
- documentação estiver disponível;
- `cargo clippy` não apresentar warnings;
- a arquitetura não estiver acoplada a um framework HTTP específico.

O agente deverá terminar apresentando:

```text
1. Arquivos criados
2. Arquivos modificados
3. APIs públicas implementadas
4. Dependências adicionadas
5. Testes executados
6. Exemplos criados
7. Decisões arquiteturais
8. Pendências
9. Próximos passos
```