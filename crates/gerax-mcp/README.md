# gerax-mcp

Implementação do **Model Context Protocol (MCP)** para o ecossistema Gerax.

Permite que aplicações Gerax exponham **Tools**, **Resources** e **Prompts**
para agentes de IA através do protocolo MCP, sem acoplar o domínio ao
transporte ou ao framework HTTP.

Versão de protocolo suportada: `2025-11-25`.

## Arquitetura

```text
Gerax (regras de negócio)
      │
      ▼
 gerax-mcp  ── Tools ▪ Resources ▪ Prompts ▪ CQRS
      │
      ▼
 MCP Dispatcher (JSON-RPC 2.0)
      │
      ▼
  Transport  ── STDIO ▪ HTTP (gerax-mcp-http)
      │
      ▼
     MCP Client  →  AI Agent
```

Princípios:

- **Sem regra de negócio** na crate.
- **Protocolo separado do transporte** (`gerax_mcp::protocol` vs
  `gerax_mcp::transport`).
- **Tools/Resources/Prompts separados do transporte** (registries).
- **Sem dependência obrigatória de `gerax-http`**.

## Instalação

```toml
[dependencies]
gerax-mcp = { path = "../gerax/crates/gerax-mcp" }
```

A feature `cqrs` (habilitada por padrão) inclui a integração com
`gerax-cqrs`.

## Uso rápido (STDIO)

```rust
use std::sync::Arc;
use async_trait::async_trait;
use gerax_mcp::{
    CallToolResult, McpError, McpServer, Tool, ToolError,
};
use serde_json::{json, Value};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn input_schema(&self) -> Value { json!({ "type": "object" }) }
    async fn call(&self, args: Value) -> Result<CallToolResult, ToolError> {
        Ok(CallToolResult::text(args["msg"].to_string()))
    }
}

#[tokio::main]
async fn main() -> Result<(), McpError> {
    McpServer::builder()
        .name("meu-servidor")
        .version("0.1.0")
        .tool(EchoTool)
        .build()
        .run_stdio()
        .await
}
```

Execute com `cargo run` e configure o cliente MCP com
`command: cargo`, `args: ["run", "--bin", "meu-servidor"]`.

## Cliente MCP

O cliente conecta a qualquer servidor MCP por um [`Transport`]
independente, negocia a sessão e expõe Tools, Resources e Prompts.

```rust
use gerax_mcp::{McpClient, StdioTransport};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Criar o cliente, 2. Conectar.
    let client = McpClient::builder()
        .name("gerax-agent")
        .version("0.1.0")
        .connect(StdioTransport::command("meu-mcp-server").build().await?)
        .await?;

    // 3. Initialize.
    client.initialize().await?;

    // 4. List Tools.
    let tools = client.list_tools().await?;

    // 5. Call Tool.
    let result = client
        .call_tool("echo", json!({ "msg": "olá" }))
        .await?;

    // 6. List Resources.
    let resources = client.list_resources().await?;

    // 7. Read Resource.
    let contents = client.read_resource("docs://manual").await?;

    // 8. List Prompts.
    let prompts = client.list_prompts().await?;

    // 9. Get Prompt.
    let prompt = client.get_prompt("hello", json!({ "name": "Ana" })).await?;

    // 10. Close.
    client.close().await?;
    Ok(())
}
```

Detalhes:

- **Requests concorrentes**: respostas fora de ordem são correlacionadas
  pelo ID JSON-RPC; cada `Future` recebe a própria resposta.
- **Caches de descoberta**: `client.tools()` / `resources()` / `prompts()`
  cacheiam a primeira listagem e são invalidados automaticamente pelas
  notifications `.../list_changed`. As listagens `list_*` sempre buscam
  no servidor.
- **Thread safety**: `McpClient<T>` é `Send + Sync` para qualquer
  transporte; compartilhe via `Arc` entre tarefas.
- **Timeout**: cada request expira após o timeout configurado no builder
  (padrão 30s).

## Tools

Implemente o trait `Tool` (nome, descrição, schema de entrada e `call`)
ou use os atalhos tipados `tool`/`tool_with` para handlers async:

```rust
use gerax_mcp::{CallToolResult, Tool, ToolError, tool};

let sum = tool(
    "sum",
    "Soma dois números",
    json!({ "type": "object", "properties": { "a": { "type": "number" } } , "required": ["a"]}),
    |args| async move {
        let a = args["a"].as_f64().ok_or_else(|| ToolError::Internal("a ausente".into()))?;
        Ok(CallToolResult::text((a + 1.0).to_string()))
    },
);
```

## Resources

```rust
use async_trait::async_trait;
use gerax_mcp::{Resource, ResourceContents, ResourceError};

struct ManualResource;

#[async_trait]
impl Resource for ManualResource {
    fn uri(&self) -> &str { "docs://manual" }
    fn name(&self) -> &str { "Manual" }
    async fn read(&self) -> Result<ResourceContents, ResourceError> {
        Ok(ResourceContents::text(self.uri(), "# Manual"))
    }
}
```

## Prompts

```rust
use async_trait::async_trait;
use gerax_mcp::{GetPromptResult, Prompt, PromptContent, PromptError, PromptMessage};

struct SaudacaoPrompt;

#[async_trait]
impl Prompt for SaudacaoPrompt {
    fn name(&self) -> &str { "saudacao" }
    async fn get(&self, _args: serde_json::Value) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::new(
            None,
            vec![PromptMessage::assistant(PromptContent::text("Olá!"))],
        ))
    }
}
```

## CQRS

Com a feature `cqrs`, comandos de domínio viram Tools MCP automaticamente:

```rust
server.register_command::<CriarAluno, _>(command_bus)?;
```

Veja [`examples/cqrs.rs`](examples/cqrs.rs).

## Segurança

- `gerax-mcp` não implementa autenticação; conecte um `McpAuthorizer`
  para autorizar cada operação usando o `McpContext` (id do request,
  método, cliente e versão do protocolo).
- Nenhuma Tool destrutiva é registrada por padrão.
- Erros retornados ao cliente são sanitizados: nunca enviamos stack
  traces, credenciais ou caminhos internos.
- No transporte STDIO, **nenhum log deve ir para o stdout** (corromperia
  o protocolo); use stderr.

## Transportes

| Transporte       | Status        |
| ---------------- | ------------- |
| STDIO            | Implementado  |
| HTTP (streamable) | Implementado em `gerax-mcp-http` |

O transporte HTTP (Streamable HTTP) fica na crate **`gerax-mcp-http`**,
que usa o `McpServer`/`McpDispatcher` desta crate sem contaminá-la:

```toml
gerax-mcp-http = { path = "crates/gerax-mcp-http" }
```

```rust
use gerax_mcp::McpServer;
use gerax_mcp_http::McpHttpServer;

let server = McpServer::builder().name("escola").version("0.1.0").build();

let app = McpHttpServer::for_server(server)
    .endpoint("/mcp")
    .origin_policy(gerax_mcp_http::OriginPolicy::DenyIfPresent)
    .build()
    .router(); // axum::Router

let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
axum::serve(listener, app).await?;
```

Regras do transporte (spec `2025-11-25`):

- `POST /mcp` envia uma mensagem JSON-RPC por vez; notifications do
  cliente são aceitas com `202`.
- `initialize` devolve o header `MCP-Session-Id`; requests seguintes
  devem repeti-lo (sem header: `400`; sessão desconhecida: `404`).
- Com `Accept: text/event-stream`, a resposta é SSE (evento "prime" +
  resposta); caso contrário um JSON único.
- `GET /mcp` abre stream SSE de notifications; `DELETE /mcp` encerra a
  sessão e permite um novo `initialize`.
- `MCP-Protocol-Version` inválido → `400`; `Origin` fora da política →
  `403`.

**Limitação deliberada:** uma instância do `McpHttpServer` mantém **uma
sessão ativa** (o lifecycle do `McpServer` é global). Use o `DELETE` para
liberar a sessão para um novo cliente.

## Exemplos

- [`examples/server.rs`](examples/server.rs) — servidor STDIO com
  Tool, Resource, Prompt e autorizador.
- [`examples/cqrs.rs`](examples/cqrs.rs) — comando `gerax-cqrs` exposto como Tool.
- [`examples/mcp_client.rs`](examples/mcp_client.rs) — cliente percorrendo
  connect → initialize → tools → resources → prompts → close.
- [`examples/mcp_client_stdio.rs`](examples/mcp_client_stdio.rs) — cliente
  STDIO com configuração de processo (`args`, `env`).
- [`gerax-mcp-http/examples/http_server.rs`](../gerax-mcp-http/examples/http_server.rs) —
  servidor HTTP (`http://127.0.0.1:3000/mcp`).

O binário de teste `test_mcp_server` (Tool `echo`, Resource `note://manual`,
Prompt `hello`) serve de alvo para os exemplos e testes de integração:

```bash
cargo run -p gerax-mcp --bin test_mcp_server
cargo run -p gerax-mcp --example mcp_client
cargo test -p gerax-mcp --test stdio_e2e
```

## Documentação

```bash
cargo doc --open -p gerax-mcp
```