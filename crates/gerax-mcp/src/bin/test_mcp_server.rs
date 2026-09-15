//! Servidor MCP mínimo para testes de integração e exemplos.
//!
//! Fala MCP sobre STDIO, expondo uma Tool (`echo`), um Resource
//! (`note://manual`) e um Prompt (`hello`). Use como alvo de
//! [`StdioTransport::command`](gerax_mcp::StdioTransport):

use async_trait::async_trait;
use serde_json::{Value, json};

use gerax_mcp::prompt::{
    GetPromptResult, Prompt, PromptArgument, PromptContent, PromptError, PromptMessage,
};
use gerax_mcp::resource::StaticResource;
use gerax_mcp::tool::{CallToolResult, tool};

struct HelloPrompt;

#[async_trait]
impl Prompt for HelloPrompt {
    fn name(&self) -> &str {
        "hello"
    }

    fn arguments(&self) -> Vec<PromptArgument> {
        vec![PromptArgument::new("name", Some("quem cumprimentar"), true)]
    }

    async fn get(&self, _args: Value) -> Result<GetPromptResult, PromptError> {
        Ok(GetPromptResult::new(
            Some("Saudação".to_owned()),
            vec![PromptMessage::assistant(PromptContent::text("Olá, mundo!"))],
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let echo = tool(
        "echo",
        "Ecoa os argumentos recebidos",
        json!({ "type": "object" }),
        |args: Value| async move { Ok(CallToolResult::text(format!("eco: {args}"))) },
    );

    let manual = StaticResource::text(
        "note://manual",
        "manual",
        Some("Conteúdo do manual de teste"),
        None::<&str>,
        "conteúdo do manual",
    );

    let server = gerax_mcp::McpServer::builder()
        .name("test-mcp-server")
        .version("0.1.0")
        .instructions("Servidor de teste do cliente MCP do Gerax.")
        .tool(echo)
        .resource(manual)
        .prompt(HelloPrompt)
        .build();

    server.run_stdio().await.map_err(|error| error.into())
}
