# Skill: Implementar módulo cliente MCP — `gerax-mcp`

## Objetivo

Implementar o módulo cliente MCP (`client`) dentro da crate `gerax-mcp`.

O cliente deverá permitir que aplicações Gerax:

- conectem-se a servidores MCP;
- executem o handshake `initialize`;
- negociem a versão do protocolo;
- descubram Tools;
- executem Tools remotas;
- descubram Resources;
- leiam Resources;
- descubram Prompts;
- obtenham Prompts;
- recebam notifications;
- suportem requests concorrentes;
- correlacionem requests e responses através de JSON-RPC IDs;
- funcionem independentemente do transporte;
- suportem STDIO inicialmente;
- permitir futura integração HTTP;
- expor Tools MCP remotas para `gerax-ai`;
- permitir conexão simultânea com múltiplos servidores MCP.

---

# 1. Contexto arquitetural

O módulo será parte de:

```text
gerax-mcp/
└── src/
    ├── client/
    │   ├── mod.rs
    │   ├── client.rs
    │   ├── session.rs
    │   ├── request_manager.rs
    │   ├── capabilities.rs
    │   ├── remote_tool.rs
    │   └── error.rs
    │
    ├── protocol/
    ├── server/
    ├── tool/
    ├── resource/
    ├── prompt/
    └── transport/
```

A arquitetura deverá ser:

```text
                 McpClient
                     │
                     ▼
              RequestManager
                     │
                     ▼
                 Transport
                     │
          ┌──────────┴──────────┐
          ▼                     ▼
       STDIO                   HTTP
          │                     │
          ▼                     ▼
     MCP Server            MCP Server
```

O `McpClient` não deve conhecer detalhes específicos de STDIO, HTTP, TCP ou qualquer framework web.

---

# 2. Princípios

O agente deverá seguir obrigatoriamente:

1. Rust idiomático.
2. Rust edition 2024.
3. Rust 1.96.1 ou superior conforme o `rust-version` do workspace.
4. `async/await`.
5. `Send + Sync` onde apropriado.
6. Separação entre protocolo, cliente e transporte.
7. Requests concorrentes.
8. Correlação correta entre request e response.
9. Não bloquear o runtime Tokio.
10. Não usar `unwrap()` ou `expect()` em código de biblioteca.
11. Erros tipados.
12. API pública pequena.
13. Não duplicar os tipos existentes no módulo `protocol`.
14. Não acoplar `McpClient` a Actix, Axum, Poem ou Salvo.
15. Não implementar regras de negócio dentro do cliente.
16. Preparar integração futura com `gerax-ai`.
17. Escrever testes antes de considerar cada fase concluída.

---

# 3. Inspecionar o workspace antes de implementar

Antes de escrever código, o agente deverá analisar:

```text
Cargo.toml
gerax-core
gerax-http
gerax-ai
gerax-cqrs
gerax-mcp
```

Especialmente:

```text
gerax-mcp/src/protocol
gerax-mcp/src/transport
```

Caso `gerax-mcp` ainda não exista, criar a crate conforme a arquitetura existente.

O agente deverá verificar se já existem:

```text
JsonRpcRequest
JsonRpcResponse
JsonRpcId
JsonRpcError
ServerCapabilities
Tool
Resource
Prompt
Transport
```

Se existirem, reutilizá-los.

Não criar versões paralelas.

---

# FASE 1 — Estrutura do módulo client

Criar:

```text
gerax-mcp/src/client/
├── mod.rs
├── client.rs
├── session.rs
├── request_manager.rs
├── capabilities.rs
├── remote_tool.rs
└── error.rs
```

Em:

```rust
pub mod client;
pub mod session;
pub mod request_manager;
pub mod capabilities;
pub mod remote_tool;
pub mod error;
```

Expor uma API limpa:

```rust
pub use client::McpClient;
pub use session::McpSession;
pub use remote_tool::RemoteTool;
```

---

# FASE 2 — `McpClient`

Criar o tipo:

```rust
pub struct McpClient<T> {
    transport: T,
    ...
}
```

O transporte deverá ser genérico:

```rust
pub struct McpClient<T>
where
    T: Transport,
{
    ...
}
```

Não utilizar um transporte concreto dentro do cliente.

A API inicial deverá permitir:

```rust
let client = McpClient::builder()
    .name("gerax-agent")
    .version("0.1.0")
    .build();
```

Depois:

```rust
let client = client
    .connect(transport)
    .await?;
```

ou uma API equivalente que preserve separação entre construção e conexão.

O agente deverá escolher a API mais idiomática considerando a implementação existente de `Transport`.

---

# FASE 3 — Client Builder

Implementar:

```rust
pub struct McpClientBuilder {
    name: String,
    version: String,
    capabilities: ClientCapabilities,
}
```

Permitir:

```rust
McpClient::builder()
    .name("gerax-agent")
    .version("0.1.0")
    .build();
```

Valores obrigatórios:

```text
name
version
```

Capabilities devem possuir defaults seguros.

---

# FASE 4 — Client Session

Criar:

```rust
pub struct McpSession {
    pub protocol_version: String,
    pub server_info: ImplementationInfo,
    pub server_capabilities: ServerCapabilities,
}
```

A sessão deverá representar o estado negociado com o servidor.

Estados conceituais:

```text
Created
   │
   ▼
Connected
   │
   ▼
Initializing
   │
   ▼
Initialized
   │
   ▼
Closed
```

Evitar expor estados internos desnecessariamente.

---

# FASE 5 — Initialize

Implementar:

```rust
pub async fn initialize(
    &self,
) -> Result<McpSession, McpClientError>;
```

O request deverá utilizar o método MCP:

```text
initialize
```

Enviar:

```text
protocolVersion
capabilities
clientInfo
```

Receber:

```text
protocolVersion
capabilities
serverInfo
```

Após sucesso, enviar:

```text
notifications/initialized
```

Fluxo:

```text
Client
  │
  │ initialize
  ▼
Server
  │
  │ InitializeResult
  ▼
Client
  │
  │ notifications/initialized
  ▼
Server
```

---

# FASE 6 — Controle do estado

O cliente não deverá permitir operações MCP que dependam de uma sessão antes do `initialize`.

Por exemplo:

```rust
client.list_tools().await?;
```

antes de:

```rust
client.initialize().await?;
```

deverá produzir:

```rust
McpClientError::NotInitialized
```

Não utilizar panic.

---

# FASE 7 — Request Manager

Esta é uma das partes mais importantes da implementação.

Criar:

```rust
pub struct RequestManager {
    ...
}
```

Responsabilidades:

- gerar IDs;
- enviar requests;
- armazenar requests pendentes;
- receber responses;
- correlacionar response com request;
- acordar o `Future` correto;
- tratar timeout;
- detectar response desconhecida;
- tratar encerramento do transporte.

---

# FASE 8 — Request IDs

Implementar geração monotônica de IDs:

```text
1
2
3
4
5
...
```

A implementação deverá ser segura para chamadas concorrentes.

Não usar:

```rust
static mut
```

Preferir:

```rust
AtomicU64
```

ou mecanismo equivalente.

---

# FASE 9 — Requests concorrentes

O cliente deverá permitir:

```rust
let a = client.call_tool(...);
let b = client.call_tool(...);
let c = client.list_resources();

let (a, b, c) = tokio::join!(a, b, c);
```

O servidor poderá responder fora de ordem:

```text
request 1 ──────────────►
request 2 ──────────────►
request 3 ──────────────►

response 2 ◄─────────────
response 3 ◄─────────────
response 1 ◄─────────────
```

O `RequestManager` deverá entregar:

```text
response 1 → Future 1
response 2 → Future 2
response 3 → Future 3
```

---

# FASE 10 — Pending Requests

Usar estrutura conceitualmente equivalente a:

```rust
HashMap<JsonRpcId, PendingRequest>
```

onde:

```rust
struct PendingRequest {
    sender: oneshot::Sender<JsonRpcResponse>,
}
```

A implementação concreta poderá ser diferente.

Requisitos:

- thread-safe;
- async-friendly;
- sem lock durante operações lentas;
- remoção do request após resposta;
- remoção após timeout;
- limpeza no fechamento do cliente.

---

# FASE 11 — Response Dispatcher

Criar um loop interno responsável por receber mensagens:

```text
Transport
    │
    ▼
ResponseDispatcher
    │
    ├── Response ID 1 → Pending #1
    ├── Response ID 2 → Pending #2
    ├── Response ID 3 → Pending #3
    │
    └── Notification → NotificationHandler
```

O dispatcher não deve executar Tools diretamente.

Ele apenas distribui mensagens.

---

# FASE 12 — Notifications

O cliente deverá conseguir receber notifications sem request correspondente.

Exemplo:

```text
notifications/tools/list_changed
```

Não tentar procurar:

```text
pending[id]
```

para notifications.

Criar uma abstração:

```rust
pub trait NotificationHandler {
    ...
}
```

ou mecanismo equivalente.

Inicialmente pode haver um callback/event stream.

O design deverá permitir futuramente:

```rust
client.subscribe_notifications()
```

---

# FASE 13 — Timeout

Permitir timeout configurável:

```rust
McpClient::builder()
    .request_timeout(Duration::from_secs(30))
```

Cada request deverá possuir timeout.

Após timeout:

```rust
McpClientError::Timeout
```

O request deverá ser removido de `pending`.

---

# FASE 14 — Cancelamento

O design deverá permitir cancelamento futuro.

Não é obrigatório implementar imediatamente todo mecanismo de cancelamento MCP.

Porém:

```text
Future dropped
```

não deverá causar:

- deadlock;
- vazamento permanente;
- corrupção do `pending`;
- travamento do dispatcher.

---

# FASE 15 — Tools

Implementar:

```rust
pub async fn list_tools(
    &self,
) -> Result<Vec<ToolDefinition>, McpClientError>;
```

Usar:

```text
tools/list
```

Resultado deverá preservar:

```text
name
description
inputSchema
```

---

# FASE 16 — Paginação de Tools

Caso o protocolo utilizado suportar paginação, implementar:

```text
nextCursor
```

Criar API de alto nível:

```rust
client.list_tools().await?;
```

que deverá buscar todas as páginas quando apropriado.

Opcionalmente disponibilizar API paginada:

```rust
client.list_tools_page(cursor).await?;
```

Evitar esconder paginação de maneira que cause loops infinitos.

Validar:

```text
cursor repetido
cursor inválido
servidor malformado
```

---

# FASE 17 — Call Tool

Implementar:

```rust
pub async fn call_tool(
    &self,
    name: impl Into<String>,
    arguments: serde_json::Value,
) -> Result<CallToolResult, McpClientError>;
```

Usar:

```text
tools/call
```

Exemplo:

```rust
let result = client
    .call_tool(
        "create_aluno",
        json!({
            "nome": "João",
            "email": "joao@example.com"
        }),
    )
    .await?;
```

---

# FASE 18 — RemoteTool

Criar:

```rust
pub struct RemoteTool {
    ...
}
```

Ele representa uma Tool disponibilizada remotamente por um MCP Server.

Deve conter:

```text
name
description
input_schema
client
```

Exemplo:

```rust
let tools = client.remote_tools().await?;

for tool in tools {
    println!("{}", tool.name());
}
```

E:

```rust
tool.call(arguments).await?;
```

O `RemoteTool` não deve possuir lógica MCP própria.

Ele delega ao `McpClient`.

---

# FASE 19 — RemoteTool como abstração

Projetar `RemoteTool` para futuramente implementar uma abstração comum de Tools do Gerax.

Objetivo futuro:

```text
             Tool
              ▲
       ┌──────┴──────┐
       │             │
   LocalTool     RemoteTool
                     │
                     ▼
                 McpClient
```

Não acoplar essa implementação a `gerax-ai` ainda se isso gerar dependência circular.

Caso seja necessário, criar uma trait neutra em `gerax-core`.

---

# FASE 20 — Resources

Implementar:

```rust
pub async fn list_resources(
    &self,
) -> Result<Vec<ResourceDefinition>, McpClientError>;
```

Usar:

```text
resources/list
```

Implementar:

```rust
pub async fn read_resource(
    &self,
    uri: &str,
) -> Result<ReadResourceResult, McpClientError>;
```

Usar:

```text
resources/read
```

---

# FASE 21 — Resource Templates

Se suportado pela versão do protocolo utilizada pelo Gerax, implementar descoberta de:

```text
resources/templates/list
```

Manter essa funcionalidade separada de:

```text
resources/list
```

---

# FASE 22 — Prompts

Implementar:

```rust
pub async fn list_prompts(
    &self,
) -> Result<Vec<PromptDefinition>, McpClientError>;
```

Usar:

```text
prompts/list
```

Implementar:

```rust
pub async fn get_prompt(
    &self,
    name: &str,
    arguments: serde_json::Value,
) -> Result<GetPromptResult, McpClientError>;
```

Usar:

```text
prompts/get
```

---

# FASE 23 — Capabilities

Criar APIs:

```rust
client.server_capabilities()
client.server_info()
client.protocol_version()
```

Permitir verificar:

```rust
if client.supports_tools() {
    ...
}
```

ou API equivalente.

O cliente deverá rejeitar operações incompatíveis com as capabilities anunciadas pelo servidor quando isso for semanticamente necessário.

---

# FASE 24 — Server Information

Disponibilizar:

```rust
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}
```

O cliente deverá conseguir consultar:

```rust
let server = client.server_info();
```

Sem fazer nova chamada de rede.

---

# FASE 25 — Transport abstraction

O cliente deverá depender apenas de:

```rust
Transport
```

Nunca diretamente de:

```text
Tokio process
HTTP client
Actix
Axum
Reqwest
```

Esses detalhes pertencem aos transports.

Arquitetura:

```text
McpClient<T>
      │
      ▼
Transport
      │
 ┌────┴─────┐
 ▼          ▼
STDIO      HTTP
```

---

# FASE 26 — STDIO Client

Implementar um transporte cliente para iniciar um servidor MCP local.

Exemplo conceitual:

```rust
let transport = StdioTransport::command(
    "my-mcp-server",
)
.args(["--stdio"])
.build()
.await?;
```

Depois:

```rust
let client = McpClient::builder()
    .name("gerax-agent")
    .version("0.1.0")
    .connect(transport)
    .await?;

client.initialize().await?;
```

O transporte deverá gerenciar:

```text
stdin
stdout
stderr
process
```

`stderr` não deverá ser interpretado como protocolo MCP.

---

# FASE 27 — Process Lifecycle

O transporte STDIO deverá lidar com:

```text
process started
process exited
stdin closed
stdout closed
broken pipe
```

Erros deverão ser convertidos para:

```rust
TransportError
```

O cliente deverá acordar requests pendentes quando o processo morrer.

Nenhum `Future` deverá permanecer pendurado indefinidamente.

---

# FASE 28 — Connection Lifecycle

Implementar:

```rust
pub async fn close(&self) -> Result<(), McpClientError>;
```

Após fechamento:

```rust
client.call_tool(...).await
```

deverá retornar:

```rust
McpClientError::Closed
```

e não tentar utilizar o transporte.

---

# FASE 29 — Reconnection

Não implementar reconexão automática inicialmente.

Porém, a arquitetura deverá permitir futuramente:

```text
Disconnected
     │
     ▼
Reconnect
     │
     ▼
Initialize
     │
     ▼
Initialized
```

Não esconder reconexão automática dentro de `McpClient` nesta primeira versão.

---

# FASE 30 — Error Model

Criar:

```rust
pub enum McpClientError {
    NotInitialized,
    AlreadyInitialized,
    Closed,
    Timeout,
    Transport(TransportError),
    Protocol(...),
    JsonRpc(...),
    Server(...),
    InvalidResponse(...),
    Unsupported(...),
}
```

A implementação final deverá adaptar os tipos existentes no Gerax em vez de duplicá-los.

Não expor detalhes internos desnecessários.

---

# FASE 31 — Error propagation

Um erro MCP deverá preservar informação suficiente para diagnóstico:

```text
request method
request id
server error code
server error message
```

Exemplo:

```text
Tool call failed:
  method: tools/call
  id: 42
  code: -32602
  message: Invalid arguments
```

Não incluir secrets ou credenciais nos erros.

---

# FASE 32 — Thread safety

Determinar cuidadosamente quais componentes serão:

```text
Send
Sync
Clone
```

Objetivo:

```rust
Arc<McpClient<_>>
```

poder ser compartilhado entre tarefas.

Exemplo:

```rust
let client = Arc::new(client);

tokio::spawn({
    let client = client.clone();
    async move {
        client.call_tool(...).await
    }
});
```

Evitar `Mutex` global envolvendo toda a operação do cliente.

---

# FASE 33 — Cache de Tools

O cliente poderá manter cache local:

```rust
struct ToolCache {
    tools: Vec<RemoteTool>,
}
```

API:

```rust
client.tools().await?;
```

deverá permitir que o agente reutilize as definições.

Entretanto:

```text
tools/list_changed
```

deverá invalidar o cache.

Não tornar o cache obrigatório se complicar a implementação.

---

# FASE 34 — Cache de Resources e Prompts

Aplicar o mesmo conceito, se apropriado:

```text
Resource cache
Prompt cache
```

As caches deverão ser invalidadas por notifications correspondentes.

---

# FASE 35 — MCP Tool Provider

Preparar uma camada de integração para `gerax-ai`.

Objetivo:

```text
McpClient
     │
     ▼
McpToolProvider
     │
     ▼
ToolProvider
     │
     ▼
AI Agent
```

Criar adapter somente se houver uma trait `ToolProvider` já existente.

Não criar uma dependência circular entre:

```text
gerax-ai
gerax-mcp
```

Se necessário, a abstração deverá ficar em uma crate inferior.

---

# FASE 36 — Multi-server

Criar futuramente:

```rust
pub struct McpClientPool {
    ...
}
```

Não é obrigatório implementar na primeira versão completa do cliente.

Mas a arquitetura deverá permitir:

```text
Agent
 │
 ├── McpClient → filesystem
 ├── McpClient → database
 ├── McpClient → github
 └── Local tools
```

O pool poderá futuramente agregar:

```text
tools
resources
prompts
```

de vários servidores.

---

# FASE 37 — Testes unitários

Criar testes para:

```text
ClientBuilder
Request ID
Session
Initialize
Capabilities
NotInitialized
AlreadyInitialized
Closed
Timeout
Server errors
Protocol errors
```

---

# FASE 38 — Testes do RequestManager

Testar obrigatoriamente:

```text
single request
multiple concurrent requests
out-of-order responses
unknown response ID
duplicate response
timeout
transport close
pending cleanup
```

Teste conceitual:

```text
request A → ID 1
request B → ID 2
request C → ID 3

response C → ID 3
response A → ID 1
response B → ID 2
```

Verificar que cada Future recebe sua própria resposta.

---

# FASE 39 — Mock Transport

Criar:

```rust
MockTransport
```

para testes.

Permitir:

```text
client
   │
   ▼
MockTransport
   │
   ▼
fake MCP server
```

Isso deverá evitar iniciar processos reais nos testes unitários.

---

# FASE 40 — Integration Test

Criar um servidor MCP mínimo de teste:

```text
Test MCP Server
├── Tool
├── Resource
└── Prompt
```

O cliente deverá:

```text
connect
initialize
list tools
call tool
list resources
read resource
list prompts
get prompt
close
```

---

# FASE 41 — STDIO Integration Test

Criar um servidor de teste executável.

Fluxo:

```text
McpClient
    │
    ▼
StdioTransport
    │
    ▼
test-mcp-server
```

Validar comunicação real.

---

# FASE 42 — Concurrent Tool Calls

Criar teste:

```rust
let a = client.call_tool("tool_a", json!({}));
let b = client.call_tool("tool_b", json!({}));
let c = client.call_tool("tool_c", json!({}));

let (a, b, c) = tokio::join!(a, b, c);
```

O servidor deverá responder propositalmente em ordem diferente.

Todos deverão terminar corretamente.

---

# FASE 43 — Notification Test

Simular:

```text
notifications/tools/list_changed
```

Verificar invalidação da Tool cache.

Depois:

```rust
client.list_tools().await?;
```

deverá buscar novamente as Tools.

---

# FASE 44 — API Ergonomics

Avaliar se o uso final é simples.

Objetivo:

```rust
let client = McpClient::builder()
    .name("gerax-agent")
    .version("0.1.0")
    .connect(transport)
    .await?;

client.initialize().await?;

let tools = client.list_tools().await?;

let result = client
    .call_tool("search", json!({
        "query": "Rust"
    }))
    .await?;
```

Evitar APIs excessivamente verbosas.

---

# FASE 45 — Documentação

Documentar:

```text
McpClient
McpClientBuilder
McpSession
RequestManager
RemoteTool
McpClientError
```

Criar exemplos:

```text
examples/mcp_client.rs
examples/mcp_client_stdio.rs
```

O README deverá demonstrar:

```text
1. Criar client
2. Conectar
3. Initialize
4. List Tools
5. Call Tool
6. List Resources
7. Read Resource
8. List Prompts
9. Get Prompt
10. Close
```

---

# FASE 46 — Segurança

Não registrar automaticamente:

```text
authorization headers
tokens
API keys
credentials
tool arguments sensíveis
resource contents
```

Logs deverão evitar vazamento de dados.

Preparar extensão futura:

```rust
pub trait McpClientMiddleware {
    ...
}
```

Somente implementar se houver necessidade real.

---

# FASE 47 — Observabilidade

Preparar integração futura com tracing.

Exemplo conceitual:

```text
mcp.request
mcp.response
mcp.tool.call
mcp.resource.read
mcp.prompt.get
```

Não acoplar o cliente ao sistema de observabilidade.

Se `tracing` for utilizado, mantê-lo opcional quando possível.

---

# FASE 48 — Quality Gate

Executar:

```bash
cargo fmt --all -- --check
cargo check -p gerax-mcp
cargo test -p gerax-mcp
cargo clippy -p gerax-mcp --all-targets --all-features -- -D warnings
cargo doc -p gerax-mcp --no-deps
```

Depois:

```bash
cargo check --workspace
cargo test --workspace
```

Corrigir todos os erros antes de finalizar.

---

# FASE 49 — Architectural Review

Verificar:

## Cliente

```text
McpClient
```

não deve conhecer:

```text
Actix
Axum
Poem
Salvo
```

## Transporte

```text
Transport
```

é responsável pela comunicação.

## Protocolo

```text
protocol/
```

é responsável pelos tipos MCP/JSON-RPC.

## Cliente

```text
client/
```

é responsável por:

```text
session
requests
responses
capabilities
remote operations
```

## AI

`gerax-ai` deverá consumir o cliente através de uma abstração apropriada.

---

# Arquitetura final esperada

```text
                         AI Agent
                            │
                            ▼
                     ┌─────────────┐
                     │ gerax-ai    │
                     └──────┬──────┘
                            │
                       ToolProvider
                            │
                            ▼
                     ┌─────────────┐
                     │  McpClient  │
                     └──────┬──────┘
                            │
                     RequestManager
                            │
                    ┌───────┴────────┐
                    │                │
                    ▼                ▼
              MCP Protocol       Notifications
                    │
                    ▼
                Transport
                    │
          ┌─────────┴──────────┐
          ▼                    ▼
       STDIO                  HTTP
          │                    │
          ▼                    ▼
    MCP Server A          MCP Server B
          │                    │
     ┌────┼────┐          ┌────┼────┐
     ▼    ▼    ▼          ▼    ▼    ▼
   Tools Res Prompts    Tools Res Prompts
```

---

# Definition of Done

O módulo cliente será considerado concluído quando:

- [ ] `McpClient` estiver implementado;
- [ ] Builder estiver implementado;
- [ ] Session estiver implementada;
- [ ] `initialize` funcionar;
- [ ] `notifications/initialized` funcionar;
- [ ] Request IDs forem gerados corretamente;
- [ ] requests concorrentes funcionarem;
- [ ] responses fora de ordem funcionarem;
- [ ] timeout funcionar;
- [ ] pending requests forem limpos;
- [ ] fechamento do transporte acordar requests pendentes;
- [ ] `tools/list` funcionar;
- [ ] `tools/call` funcionar;
- [ ] `RemoteTool` estiver implementado;
- [ ] `resources/list` funcionar;
- [ ] `resources/read` funcionar;
- [ ] `prompts/list` funcionar;
- [ ] `prompts/get` funcionar;
- [ ] capabilities forem armazenadas;
- [ ] notifications forem tratadas;
- [ ] cache seja invalidado quando necessário;
- [ ] STDIO funcionar;
- [ ] MockTransport existir;
- [ ] testes concorrentes existirem;
- [ ] testes de integração existirem;
- [ ] documentação existir;
- [ ] `cargo clippy` passar sem warnings;
- [ ] nenhuma dependência desnecessária for introduzida;
- [ ] nenhuma dependência circular for criada.

---

# Entrega final do agente

Ao concluir, o agente deverá apresentar:

```text
## Implementação

- arquivos criados
- arquivos modificados
- APIs públicas

## MCP

- initialize
- tools
- resources
- prompts
- notifications

## Concorrência

- RequestManager
- correlation IDs
- pending requests
- timeout

## Transport

- STDIO
- HTTP (se implementado)

## Integrações

- gerax-core
- gerax-ai
- gerax-cqrs

## Testes

- testes unitários
- testes concorrentes
- testes de integração
- STDIO

## Qualidade

- cargo fmt
- cargo check
- cargo test
- cargo clippy
- cargo doc

## Pendências

Lista objetiva de funcionalidades não implementadas.

## Próximos passos

Priorizar:

1. `McpClientPool`
2. `McpToolProvider`
3. integração com `gerax-ai`
4. HTTP transport
5. reconexão
6. autenticação/autorização
```

# Regra final

O agente **não deve implementar o cliente como um simples wrapper de chamadas JSON-RPC**.

O objetivo é criar uma infraestrutura de cliente MCP robusta, concorrente e reutilizável, capaz de servir como ponte entre:

```text
LLM / AI Agent
       ↓
   gerax-ai
       ↓
  McpToolProvider
       ↓
   McpClient
       ↓
   Transport
       ↓
   MCP Server
```

O `McpClient` deverá ser considerado uma infraestrutura de comunicação e não uma implementação de agente de IA.