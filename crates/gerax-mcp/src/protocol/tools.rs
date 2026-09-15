//! Mensagens MCP de Tools (`tools/list`, `tools/call`).
//!
//! Wire format conforme a especificação MCP `2025-11-25`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool::{CallToolResult, Tool};

/// Método JSON-RPC `tools/list`.
pub const TOOLS_LIST: &str = "tools/list";

/// Método JSON-RPC `tools/call`.
pub const TOOLS_CALL: &str = "tools/call";

/// Parâmetros de `tools/list`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsParams {
    /// Cursor opcional de paginação.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Metadados de uma Tool expostos em `tools/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolMetadata {
    /// Nome único da Tool.
    pub name: String,
    /// Descrição da Tool, quando disponível.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Schema JSON de entrada da Tool.
    pub input_schema: Value,
}

impl ToolMetadata {
    /// Constrói metadados a partir de uma Tool.
    pub fn from_tool(tool: &dyn Tool) -> Self {
        Self {
            name: tool.name().to_owned(),
            description: tool.description().map(str::to_owned),
            input_schema: tool.input_schema(),
        }
    }
}

/// Resultado de `tools/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// Tools disponíveis no servidor.
    pub tools: Vec<ToolMetadata>,
    /// Cursor para a próxima página, quando houver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl ListToolsResult {
    /// Cria um resultado com ferramentas e sem próxima página.
    pub fn new(tools: Vec<ToolMetadata>) -> Self {
        Self {
            tools,
            next_cursor: None,
        }
    }
}

/// Parâmetros de `tools/call`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolRequestParams {
    /// Nome da Tool a invocar.
    pub name: String,
    /// Argumentos da Tool, opcionais.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl CallToolRequestParams {
    /// Cria parâmetros de chamada de Tool.
    pub fn new(name: impl Into<String>, arguments: Option<Value>) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}

/// Resultado serializável de `tools/call` (wire format).
pub type CallToolResultJson = CallToolResult;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tools_list_result_serializes() {
        let result = ListToolsResult::new(vec![ToolMetadata {
            name: "echo".to_owned(),
            description: Some("Ecoa".to_owned()),
            input_schema: json!({ "type": "object" }),
        }]);

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["tools"][0]["name"], "echo");
        assert_eq!(value["tools"][0]["description"], "Ecoa");
        assert_eq!(
            value["tools"][0]["inputSchema"],
            json!({ "type": "object" })
        );
        assert!(value.get("nextCursor").is_none());
    }

    #[test]
    fn tools_list_result_round_trips() {
        let result = ListToolsResult::new(Vec::new());

        let json = serde_json::to_string(&result).unwrap();
        let decoded: ListToolsResult = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, result);
    }

    #[test]
    fn call_tool_params_serializes() {
        let params = CallToolRequestParams::new("echo", Some(json!({ "msg": "oi" })));

        let value = serde_json::to_value(&params).unwrap();

        assert_eq!(
            value,
            json!({ "name": "echo", "arguments": { "msg": "oi" } })
        );
    }

    #[test]
    fn call_tool_params_without_arguments_omits() {
        let params = CallToolRequestParams::new("echo", None);

        let value = serde_json::to_value(&params).unwrap();

        assert!(value.get("arguments").is_none());
    }
}
