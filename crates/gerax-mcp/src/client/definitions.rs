//! Tipos de alto nível descobertos pelo cliente MCP.
//!
//! [`ToolDefinition`], [`ResourceDefinition`], [`ResourceTemplateDefinition`]
//! e [`PromptDefinition`] são a camada desacoplada do protocolo: o cliente
//! converte os metadados do wire format ([`ToolMetadata`],
//! [`ResourceMetadata`], [`ResourceTemplateMetadata`], [`PromptMetadata`])
//! nestes tipos estáveis para a aplicação.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prompt::PromptArgument;
use crate::protocol::{PromptMetadata, ResourceMetadata, ResourceTemplateMetadata, ToolMetadata};

/// Definição de uma Tool exposta por um MCP Server.
///
/// Preserva `name`, `description` e `inputSchema` descobertos em
/// `tools/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Nome único da Tool.
    pub name: String,
    /// Descrição da Tool, quando disponível.
    pub description: Option<String>,
    /// Schema JSON de entrada da Tool.
    pub input_schema: Value,
}

impl From<ToolMetadata> for ToolDefinition {
    fn from(metadata: ToolMetadata) -> Self {
        Self {
            name: metadata.name,
            description: metadata.description,
            input_schema: metadata.input_schema,
        }
    }
}

/// Definição de um Resource exposto por um MCP Server.
///
/// Preserva os metadados descobertos em `resources/list`: `uri`, `name`,
/// `description`, `mimeType` e `size`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceDefinition {
    /// URI único do resource.
    pub uri: String,
    /// Nome do resource.
    pub name: String,
    /// Descrição do resource, quando disponível.
    pub description: Option<String>,
    /// Tipo MIME do resource, quando conhecido.
    pub mime_type: Option<String>,
    /// Tamanho em bytes, quando conhecido.
    pub size: Option<u64>,
}

impl From<ResourceMetadata> for ResourceDefinition {
    fn from(metadata: ResourceMetadata) -> Self {
        Self {
            uri: metadata.uri,
            name: metadata.name,
            description: metadata.description,
            mime_type: metadata.mime_type,
            size: metadata.size,
        }
    }
}

/// Definição de um Resource Template exposto por um MCP Server.
///
/// Preserva os metadados descobertos em `resources/templates/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceTemplateDefinition {
    /// Template de URI (ex.: `file:///{path}`).
    pub uri_template: String,
    /// Nome do template.
    pub name: String,
    /// Descrição do template, quando disponível.
    pub description: Option<String>,
    /// Tipo MIME, quando conhecido.
    pub mime_type: Option<String>,
}

impl From<ResourceTemplateMetadata> for ResourceTemplateDefinition {
    fn from(metadata: ResourceTemplateMetadata) -> Self {
        Self {
            uri_template: metadata.uri_template,
            name: metadata.name,
            description: metadata.description,
            mime_type: metadata.mime_type,
        }
    }
}

/// Definição de um Prompt exposto por um MCP Server.
///
/// Preserva os metadados descobertos em `prompts/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptDefinition {
    /// Nome único do prompt.
    pub name: String,
    /// Descrição do prompt, quando disponível.
    pub description: Option<String>,
    /// Argumentos declarados do prompt.
    pub arguments: Vec<PromptArgument>,
}

impl From<PromptMetadata> for PromptDefinition {
    fn from(metadata: PromptMetadata) -> Self {
        Self {
            name: metadata.name,
            description: metadata.description,
            arguments: metadata.arguments.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_definition_converts_from_metadata() {
        let definition = ToolDefinition::from(ToolMetadata {
            name: "create_aluno".to_owned(),
            description: Some("Cria um aluno".to_owned()),
            input_schema: json!({ "type": "object", "required": ["nome"] }),
        });

        assert_eq!(definition.name, "create_aluno");
        assert_eq!(definition.description.as_deref(), Some("Cria um aluno"));
        assert_eq!(
            definition.input_schema,
            json!({ "type": "object", "required": ["nome"] })
        );
    }

    #[test]
    fn resource_definition_converts_from_metadata() {
        let definition = ResourceDefinition::from(ResourceMetadata {
            uri: "file:///ola.txt".to_owned(),
            name: "ola.txt".to_owned(),
            description: None,
            mime_type: Some("text/plain".to_owned()),
            size: Some(8),
        });

        assert_eq!(definition.uri, "file:///ola.txt");
        assert_eq!(definition.name, "ola.txt");
        assert_eq!(definition.mime_type.as_deref(), Some("text/plain"));
        assert_eq!(definition.size, Some(8));
    }

    #[test]
    fn definitions_are_serializable() {
        let definition = ToolDefinition {
            name: "echo".to_owned(),
            description: None,
            input_schema: json!({ "type": "object" }),
        };

        let json = serde_json::to_string(&definition).unwrap();
        let decoded: ToolDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, definition);
    }

    #[test]
    fn resource_template_definition_converts_from_metadata() {
        let definition = ResourceTemplateDefinition::from(ResourceTemplateMetadata {
            uri_template: "file:///{path}".to_owned(),
            name: "Arquivo".to_owned(),
            description: Some("Qualquer arquivo local".to_owned()),
            mime_type: Some("text/plain".to_owned()),
        });

        assert_eq!(definition.uri_template, "file:///{path}");
        assert_eq!(definition.name, "Arquivo");
        assert_eq!(definition.mime_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn prompt_definition_converts_from_metadata() {
        let definition = PromptDefinition::from(PromptMetadata {
            name: "greeting".to_owned(),
            description: Some("Saudação".to_owned()),
            arguments: Some(vec![PromptArgument::new("name", Some("Nome"), true)]),
        });

        assert_eq!(definition.name, "greeting");
        assert_eq!(definition.description.as_deref(), Some("Saudação"));
        assert_eq!(definition.arguments.len(), 1);
    }

    #[test]
    fn prompt_definition_reports_empty_arguments() {
        let definition = PromptDefinition::from(PromptMetadata {
            name: "simple".to_owned(),
            description: None,
            arguments: None,
        });

        assert!(definition.arguments.is_empty());
    }
}
