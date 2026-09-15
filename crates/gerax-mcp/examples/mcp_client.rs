//! Exemplo de uso do cliente MCP do Gerax.
//!
//! Conecta ao `test_mcp_server` (binário da própria crate) e percorre o
//! fluxo completo: connect → initialize → List Tools → Call Tool →
//! List Resources → Read Resource → List Prompts → Get Prompt → Close.
//!
//! ```bash
//! cargo run -p gerax-mcp --example mcp_client
//! ```

use std::error::Error;

use gerax_mcp::{McpClient, StdioTransport};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Definido em runtime pelo Cargo para exemplos do mesmo pacote.
    let server = std::env::var("CARGO_BIN_EXE_test_mcp_server")
        .unwrap_or_else(|_| "target/debug/test_mcp_server".to_owned());

    // 1. Criar o cliente (nome e versão são obrigatórios).
    let client = McpClient::<StdioTransport>::builder()
        .name("gerax-agent")
        .version("0.1.0")
        // 2. Conectar (neste caso, via STDIO).
        .connect(StdioTransport::command(&server).build().await?)
        .await?;

    // 3. Initialize: negocia a sessão MCP.
    let session = client.initialize().await?;
    println!(
        "conectado a {} v{}",
        session.server_info.name, session.server_info.version
    );

    // 4. List Tools.
    let tools = client.list_tools().await?;
    println!(
        "tools: {}",
        tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 5. Call Tool.
    let result = client.call_tool("echo", json!({ "msg": "olá" })).await?;
    println!("tools/call -> {} conteúdo(s)", result.content.len());

    // 6. List Resources.
    let resources = client.list_resources().await?;
    println!(
        "resources: {}",
        resources
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 7. Read Resource.
    let contents = client.read_resource("note://manual").await?;
    println!("resources/read -> {} conteúdo(s)", contents.contents.len());

    // 8. List Prompts.
    let prompts = client.list_prompts().await?;
    println!(
        "prompts: {}",
        prompts
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 9. Get Prompt.
    let prompt = client.get_prompt("hello", json!({ "name": "Ana" })).await?;
    println!("prompts/get -> {} mensagem(ns)", prompt.messages.len());

    // 10. Close.
    client.close().await?;
    println!("cliente fechado.");
    Ok(())
}
