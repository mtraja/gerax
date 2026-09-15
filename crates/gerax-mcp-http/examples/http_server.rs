//! Servidor MCP via **Streamable HTTP** de exemplo.
//!
//! Executar:
//!
//! ```bash
//! cargo run -p gerax-mcp-http --example http_server
//! ```
//!
//! O endpoint fica em `http://127.0.0.1:3000/mcp`. Use um cliente MCP
//! configurado com `type: "http"` e `url: "http://127.0.0.1:3000/mcp"`,
//! ou teste com `curl`:
//!
//! ```text
//! curl -sX POST http://127.0.0.1:3000/mcp \
//!   -H 'content-type: application/json' \
//!   -H 'accept: application/json' \
//!   -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}'
//! ```

use gerax_mcp::{CallToolResult, McpServer, tool};
use gerax_mcp_http::McpHttpServer;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = McpServer::builder()
        .name("gerax-mcp-http-example")
        .version("0.1.0")
        .tool(tool(
            "echo",
            "Repete o texto recebido",
            json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            }),
            |args| async move {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned();
                Ok(CallToolResult::text(format!("eco: {text}")))
            },
        ))
        .build();

    let app = McpHttpServer::for_server(server)
        .endpoint("/mcp")
        .build()
        .router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    eprintln!("MCP HTTP em http://127.0.0.1:3000/mcp");
    axum::serve(listener, app).await?;
    Ok(())
}
