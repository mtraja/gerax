# gerax-mcp-http

Transporte **Streamable HTTP** do [Model Context Protocol](https://modelcontextprotocol.io)
(versão `2025-11-25`) para o Gerax.

A lógica MCP permanece na crate [`gerax-mcp`](../gerax-mcp): esta crate
adiciona apenas o enquadramento HTTP — sessão, `Origin`, `MCP-Protocol-Version`,
respostas `application/json` ou `text/event-stream` (SSE) e o roteamento **axum**.

```text
HTTP Client
     │
    POST/GET/DELETE /mcp
     │
     ▼
 gerax-mcp-http  (sessão, SSE, Origin, versão)
     │
     ▼
 McpDispatcher ← McpServer (gerax-mcp)
```

## Exemplo mínimo

```rust,no_run
use gerax_mcp::McpServer;
use gerax_mcp_http::McpHttpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder().name("escola").version("0.1.0").build();

    let app = McpHttpServer::for_server(server)
        .endpoint("/mcp")
        .build()
        .router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

Clientes MCP (ex.: LLM servers, `curl`) apontam para
`http://127.0.0.1:3000/mcp`.

## Comportamento (spec `2025-11-25`)

| Request                              | Resposta |
| ------------------------------------ | -------- |
| `POST` mensagem JSON-RPC             | JSON-RPC único (ou SSE com `Accept: text/event-stream`); notifications aceitas com `202` |
| `POST` sem `MCP-Session-Id`          | `400`    |
| `POST` com sessão desconhecida       | `404`    |
| `GET` (Accept: `text/event-stream`)  | Stream SSE de notifications (evento "prime" + `mcp-session-id`) |
| `DELETE` com `MCP-Session-Id`        | `204`; libera a sessão para novo `initialize` |
| `MCP-Protocol-Version` inválido      | `400`    |
| `Origin` fora da política            | `403`    |

Limitação deliberada: **uma sessão ativa por instância** do
`McpHttpServer` (o lifecycle do `McpServer` é global). Um novo cliente
pode re-inicializar após `DELETE`.

## Configuração

```rust
use gerax_mcp::McpServer;
use gerax_mcp_http::{McpHttpServer, OriginPolicy};

let app = McpHttpServer::for_server(server)
    .endpoint("/mcp")
    .origin_policy(OriginPolicy::Allowlist(vec![
        "https://app.gerax.io".to_owned(),
    ]))
    .allow_client_stream(true)
    .allow_session_delete(true)
    .build()
    .router();
```

## Segurança

- Validação do header `Origin` (configurável; `DenyIfPresent` para
  bloquear qualquer `Origin`, mitigando DNS rebinding).
- Sessions com id segura (UUIDv4).
- A autorização MCP em si continua no `McpAuthorizer` do `gerax-mcp`.

## Testes

```bash
cargo test -p gerax-mcp-http
```

## Exemplo

```bash
cargo run -p gerax-mcp-http --example http_server
```