//! Exemplo de cliente MCP sobre o transporte STDIO.
//!
//! Foca na configuração do [`StdioTransport`]: programa, argumentos,
//! variáveis de ambiente e diretório de trabalho. Executa o mesmo fluxo
//! do exemplo [`mcp_client`], contra o `test_mcp_server` da crate.
//!
//! ```bash
//! cargo run -p gerax-mcp --example mcp_client_stdio
//! ```

use std::error::Error;

use gerax_mcp::{McpClient, StdioTransport};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Definido em runtime pelo Cargo para exemplos do mesmo pacote.
    let server = std::env::var("CARGO_BIN_EXE_test_mcp_server")
        .unwrap_or_else(|_| "target/debug/test_mcp_server".to_owned());

    // O builder do transporte permite controlar a linha de comando,
    // o ambiente e o diretório de trabalho do processo filho.
    let transport = StdioTransport::command(&server)
        .args(["--mode", "agent"])
        .env("RUST_LOG", "warn")
        .build()
        .await?;

    let client = McpClient::<StdioTransport>::builder()
        .name("gerax-stdio-agent")
        .version("0.1.0")
        .connect(transport)
        .await?;

    client.initialize().await?;

    let tools = client.tools().await?;
    println!("tools (com cache): {}", tools.len());

    let result = client.call_tool("echo", json!({ "via": "stdio" })).await?;
    println!("echo -> {} conteúdo(s)", result.content.len());

    client.close().await?;
    Ok(())
}
