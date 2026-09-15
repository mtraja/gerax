//! Tool remota descoberta via um MCP Server.
//!
//! [`RemoteTool`] descreve uma Tool disponibilizada remotamente por um
//! servidor MCP e delega a execução ao [`super::mcp_client::McpClient`]
//! pela trait [`ToolCaller`]. A separação por trait mantém o
//! [`RemoteTool`] genérico em relação ao transporte e prepara a
//! integração futura com a abstração comum de Tools do Gerax
//! (`LocalTool` vs `RemoteTool`), sem acoplar a `gerax-ai`.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{CallToolResult, ToolError};

use super::definitions::ToolDefinition;
use super::error::McpClientError;

/// Alvo de execução de Tools remotas.
///
/// O [`McpClient`](super::mcp_client::McpClient) implementa esta trait;
/// a indireção mantém o [`RemoteTool`] independente do tipo concreto de
/// transporte (`McpClient<T>`).
#[async_trait]
pub trait ToolCaller: Send + Sync {
    /// Executa `tools/call` no servidor.
    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, McpClientError>;
}

/// Tool exposta remotamente por um MCP Server.
///
/// Não possui lógica MCP própria: carrega os metadados descobertos
/// (`name`, `description`, `input_schema`) e delega a invocação ao
/// cliente anexado.
#[derive(Clone)]
pub struct RemoteTool {
    definition: ToolDefinition,
    caller: Arc<dyn ToolCaller>,
}

impl RemoteTool {
    /// Cria uma Tool remota a partir da definição e do cliente que a
    /// executa.
    pub fn new(definition: ToolDefinition, caller: Arc<dyn ToolCaller>) -> Self {
        Self { definition, caller }
    }

    /// A definição descoberta no servidor.
    pub fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    /// Nome único da Tool.
    pub fn name(&self) -> &str {
        &self.definition.name
    }

    /// Descrição da Tool, quando disponível.
    pub fn description(&self) -> Option<&str> {
        self.definition.description.as_deref()
    }

    /// Schema JSON de entrada da Tool.
    pub fn input_schema(&self) -> &Value {
        &self.definition.input_schema
    }

    /// Invoca a Tool no servidor com os argumentos fornecidos.
    pub async fn call(&self, arguments: Value) -> Result<CallToolResult, McpClientError> {
        self.caller.call_tool(self.name(), arguments).await
    }
}

impl fmt::Debug for RemoteTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemoteTool")
            .field("name", &self.definition.name)
            .field("description", &self.definition.description)
            .finish_non_exhaustive()
    }
}

/// O [`RemoteTool`] é uma [`Tool`](crate::tool::Tool): nome, descrição,
/// schema e invocação compatíveis com tools locais, permitindo unificar
/// Local e Remoto na abstração comum do Gerax.
#[async_trait]
impl crate::tool::Tool for RemoteTool {
    fn name(&self) -> &str {
        self.name()
    }

    fn description(&self) -> Option<&str> {
        self.description()
    }

    fn input_schema(&self) -> Value {
        self.input_schema().clone()
    }

    async fn call(&self, arguments: Value) -> Result<CallToolResult, ToolError> {
        RemoteTool::call(self, arguments)
            .await
            .map_err(|error| ToolError::Internal(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct FakeCaller {
        captured: std::sync::Mutex<Option<(String, Value)>>,
    }

    #[async_trait]
    impl ToolCaller for FakeCaller {
        async fn call_tool(
            &self,
            name: &str,
            arguments: Value,
        ) -> Result<CallToolResult, McpClientError> {
            *self.captured.lock().unwrap() = Some((name.to_owned(), arguments.clone()));
            Ok(CallToolResult::text(format!("resultado de {name}")))
        }
    }

    #[tokio::test]
    async fn remote_tool_delegates_call_to_caller() {
        let caller = Arc::new(FakeCaller {
            captured: std::sync::Mutex::new(None),
        });
        let tool = RemoteTool::new(
            ToolDefinition {
                name: "create_aluno".to_owned(),
                description: Some("Cria um aluno".to_owned()),
                input_schema: json!({ "type": "object" }),
            },
            caller.clone(),
        );

        let args = json!({ "nome": "João" });
        let result = tool.call(args.clone()).await.unwrap();

        assert_eq!(result.content.len(), 1);
        let captured = caller.captured.lock().unwrap().clone().unwrap();
        assert_eq!(captured.0, "create_aluno");
        assert_eq!(captured.1, args);
    }

    #[tokio::test]
    async fn remote_tool_implements_tool_trait() {
        let caller = Arc::new(FakeCaller {
            captured: std::sync::Mutex::new(None),
        });
        let tool = RemoteTool::new(
            ToolDefinition {
                name: "echo".to_owned(),
                description: None,
                input_schema: json!({ "type": "object" }),
            },
            caller,
        );

        let tool: &dyn crate::tool::Tool = &tool;
        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), None);
    }
}
