//! Mensagens MCP de Prompts (`prompts/list`, `prompts/get`).
//!
//! Wire format conforme a especificação MCP `2025-11-25`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prompt::{GetPromptResult, Prompt, PromptArgument};

/// Método JSON-RPC `prompts/list`.
pub const PROMPTS_LIST: &str = "prompts/list";

/// Método JSON-RPC `prompts/get`.
pub const PROMPTS_GET: &str = "prompts/get";

/// Parâmetros de `prompts/list`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsParams {
    /// Cursor opcional de paginação.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Metadados de um Prompt expostos em `prompts/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMetadata {
    /// Nome único do prompt.
    pub name: String,
    /// Descrição do prompt, quando disponível.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Argumentos declarados do prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

impl PromptMetadata {
    /// Constrói metadados a partir de um Prompt.
    pub fn from_prompt(prompt: &dyn Prompt) -> Self {
        let arguments = prompt.arguments();
        Self {
            name: prompt.name().to_owned(),
            description: prompt.description().map(str::to_owned),
            arguments: if arguments.is_empty() {
                None
            } else {
                Some(arguments)
            },
        }
    }
}

/// Resultado de `prompts/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListPromptsResult {
    /// Prompts disponíveis no servidor.
    pub prompts: Vec<PromptMetadata>,
    /// Cursor para a próxima página, quando houver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl ListPromptsResult {
    /// Cria um resultado com prompts e sem próxima página.
    pub fn new(prompts: Vec<PromptMetadata>) -> Self {
        Self {
            prompts,
            next_cursor: None,
        }
    }
}

/// Parâmetros de `prompts/get`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptRequestParams {
    /// Nome do prompt a obter.
    pub name: String,
    /// Argumentos do prompt, opcionais.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl GetPromptRequestParams {
    /// Cria parâmetros de obtenção de prompt.
    pub fn new(name: impl Into<String>, arguments: Option<Value>) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}

/// Resultado serializável de `prompts/get` (wire format).
pub type GetPromptResultJson = GetPromptResult;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompts_list_result_serializes() {
        let result = ListPromptsResult::new(vec![PromptMetadata {
            name: "greeting".to_owned(),
            description: None,
            arguments: Some(vec![PromptArgument::new("name", Some("Nome"), true)]),
        }]);

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["prompts"][0]["name"], "greeting");
        assert_eq!(value["prompts"][0]["arguments"][0]["name"], "name");
        assert!(value.get("nextCursor").is_none());
    }

    #[test]
    fn prompts_list_result_omits_empty_arguments() {
        let result = ListPromptsResult::new(vec![PromptMetadata {
            name: "simple".to_owned(),
            description: None,
            arguments: None,
        }]);

        let value = serde_json::to_value(&result).unwrap();

        assert!(value["prompts"][0].get("arguments").is_none());
    }

    #[test]
    fn get_prompt_params_serializes() {
        let params = GetPromptRequestParams::new("greeting", Some(json!({ "name": "Ana" })));

        let value = serde_json::to_value(&params).unwrap();

        assert_eq!(
            value,
            json!({ "name": "greeting", "arguments": { "name": "Ana" } })
        );
    }
}
