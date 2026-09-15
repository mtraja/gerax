//! Teste de integração: Tool CQRS via MCP.
//!
//! Fluxo: Client → Dispatcher → McpServer → Tool → CommandBus → Handler.

use std::sync::Arc;

use async_trait::async_trait;
use gerax_cqrs::{
    Command, CommandBus, CommandHandler, CommandMetadata, CqrsError, HandlerRegistry,
};
use gerax_mcp::{
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, MCP_PROTOCOL_VERSION, McpDispatcher,
    McpServer,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize)]
struct CreateAluno {
    nome: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct Aluno {
    id: u64,
    nome: String,
}

impl gerax_cqrs::Message for CreateAluno {
    type Output = Aluno;
}

impl Command for CreateAluno {}

impl CommandMetadata for CreateAluno {
    const NAME: &'static str = "create_aluno";
}

struct CreateAlunoHandler;

#[async_trait]
impl CommandHandler for CreateAlunoHandler {
    type Command = CreateAluno;

    async fn handle(&self, command: CreateAluno) -> Result<Aluno, CqrsError> {
        Ok(Aluno {
            id: 1,
            nome: command.nome,
        })
    }
}

fn command_bus() -> CommandBus {
    let mut registry = HandlerRegistry::new();
    registry.register_command(CreateAlunoHandler).unwrap();
    CommandBus::new(Arc::new(registry))
}

fn init_request() -> JsonRpcRequest {
    JsonRpcRequest::new(
        1,
        "initialize",
        Some(json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        })),
    )
}

async fn init(dispatcher: &McpDispatcher) {
    dispatcher
        .dispatch(JsonRpcMessage::Request(init_request()))
        .await
        .unwrap();
    dispatcher
        .dispatch(JsonRpcMessage::Notification(JsonRpcNotification::new(
            "notifications/initialized",
            None,
        )))
        .await;
}

#[tokio::test]
async fn should_call_create_aluno_tool() {
    let server = McpServer::builder().name("escola").version("1.0").build();
    server
        .register_command::<CreateAluno, _>(command_bus())
        .unwrap();

    let dispatcher = McpDispatcher::new(Arc::new(server.clone()));

    init(&dispatcher).await;

    let response = dispatcher
        .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
            2,
            "tools/call",
            Some(json!({
                "name": "create_aluno",
                "arguments": { "nome": "Ana", "email": "ana@gerax.io" }
            })),
        )))
        .await
        .unwrap();

    let result = response.result.as_ref().unwrap();
    assert!(!result["isError"].as_bool().unwrap_or(false));
    assert_eq!(result["structuredContent"]["id"], 1);
    assert_eq!(result["structuredContent"]["nome"], "Ana");
}

#[tokio::test]
async fn tools_list_exposes_registered_command() {
    let server = McpServer::builder().build();
    server
        .register_command::<CreateAluno, _>(command_bus())
        .unwrap();

    let dispatcher = McpDispatcher::new(Arc::new(server.clone()));

    init(&dispatcher).await;

    let response = dispatcher
        .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
            3,
            "tools/list",
            None,
        )))
        .await
        .unwrap();

    let result = response.result.as_ref().unwrap();
    assert_eq!(result["tools"][0]["name"], "create_aluno");

    let capabilities = server.capabilities();
    assert!(capabilities.tools.is_some());
}
