//! Tipos de conteúdo e resultado de invocação de Tools.

use serde::{Deserialize, Serialize};
use serde_json::Value;

fn is_false(value: &bool) -> bool {
    !*value
}

/// Conteúdo de uma resposta de Tool, conforme a especificação MCP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContent {
    /// Conteúdo textual.
    Text(TextContent),
    /// Conteúdo de imagem.
    Image(ImageContent),
}

impl From<String> for ToolContent {
    fn from(text: String) -> Self {
        Self::Text(TextContent::new(text))
    }
}

impl From<&str> for ToolContent {
    fn from(text: &str) -> Self {
        Self::Text(TextContent::new(text))
    }
}

/// Conteúdo textual de uma Tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    /// Texto do conteúdo.
    pub text: String,
}

impl TextContent {
    /// Cria um conteúdo textual.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Conteúdo de imagem de uma Tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    /// Dados da imagem codificados em base64.
    pub data: String,
    /// Tipo MIME da imagem (ex.: `image/png`).
    pub mime_type: String,
}

impl ImageContent {
    /// Cria um conteúdo de imagem.
    pub fn new(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }
}

/// Resultado de uma chamada de Tool, conforme a especificação MCP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// Conteúdos produzidos pela Tool.
    #[serde(default)]
    pub content: Vec<ToolContent>,
    /// Indica se a execução falhou.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_error: bool,
    /// Conteúdo estruturado (não textual), opcional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
}

impl CallToolResult {
    /// Cria um resultado vazio.
    pub fn empty() -> Self {
        Self {
            content: Vec::new(),
            is_error: false,
            structured_content: None,
        }
    }

    /// Cria um resultado com um único conteúdo textual.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::from(text.into())],
            is_error: false,
            structured_content: None,
        }
    }

    /// Cria um resultado de erro com conteúdo textual.
    pub fn text_error(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::from(text.into())],
            is_error: true,
            structured_content: None,
        }
    }

    /// Cria um resultado com `structuredContent` apenas.
    pub fn structured(value: Value) -> Self {
        Self {
            content: Vec::new(),
            is_error: false,
            structured_content: Some(value),
        }
    }
}
