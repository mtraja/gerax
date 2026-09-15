//! Mensagens MCP de Resources (`resources/list`, `resources/read`).
//!
//! Wire format conforme a especificação MCP `2025-11-25`.

use serde::{Deserialize, Serialize};

use crate::resource::ResourceContents;

/// Método JSON-RPC `resources/list`.
pub const RESOURCES_LIST: &str = "resources/list";

/// Método JSON-RPC `resources/read`.
pub const RESOURCES_READ: &str = "resources/read";

/// Método JSON-RPC `resources/templates/list`.
pub const RESOURCES_TEMPLATES_LIST: &str = "resources/templates/list";

/// Parâmetros de `resources/list`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesParams {
    /// Cursor opcional de paginação.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Metadados de um Resource expostos em `resources/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMetadata {
    /// URI único do resource.
    pub uri: String,
    /// Nome do resource.
    pub name: String,
    /// Descrição do resource, quando disponível.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tipo MIME do resource, quando conhecido.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Tamanho em bytes, quando conhecido.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

impl ResourceMetadata {
    /// Constrói metadados a partir de um Resource.
    pub fn from_resource(resource: &dyn crate::resource::Resource) -> Self {
        Self {
            uri: resource.uri().to_owned(),
            name: resource.name().to_owned(),
            description: resource.description().map(str::to_owned),
            mime_type: resource.mime_type().map(str::to_owned),
            size: None,
        }
    }
}

/// Resultado de `resources/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourcesResult {
    /// Resources disponíveis no servidor.
    pub resources: Vec<ResourceMetadata>,
    /// Cursor para a próxima página, quando houver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl ListResourcesResult {
    /// Cria um resultado com resources e sem próxima página.
    pub fn new(resources: Vec<ResourceMetadata>) -> Self {
        Self {
            resources,
            next_cursor: None,
        }
    }
}

/// Parâmetros de `resources/read`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceRequestParams {
    /// URI do resource a ler.
    pub uri: String,
}

/// Resultado de `resources/read`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadResourceResult {
    /// Conteúdos do resource.
    pub contents: Vec<ResourceContents>,
}

impl ReadResourceResult {
    /// Cria um resultado com um único conteúdo.
    pub fn new(contents: Vec<ResourceContents>) -> Self {
        Self { contents }
    }
}

/// Parâmetros de `resources/templates/list`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesParams {
    /// Cursor opcional de paginação.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Metadados de um Resource Template exposto em `resources/templates/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTemplateMetadata {
    /// Template de URI do resource (ex.: `file:///{path}`).
    pub uri_template: String,
    /// Nome do template.
    pub name: String,
    /// Descrição do template, quando disponível.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tipo MIME, quando conhecido.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// Resultado de `resources/templates/list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResourceTemplatesResult {
    /// Templates de resources disponíveis no servidor.
    pub resource_templates: Vec<ResourceTemplateMetadata>,
    /// Cursor para a próxima página, quando houver.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl ListResourceTemplatesResult {
    /// Cria um resultado sem próxima página.
    pub fn new(resource_templates: Vec<ResourceTemplateMetadata>) -> Self {
        Self {
            resource_templates,
            next_cursor: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resources_list_result_serializes() {
        let result = ListResourcesResult::new(vec![ResourceMetadata {
            uri: "file:///ola.txt".to_owned(),
            name: "ola.txt".to_owned(),
            description: None,
            mime_type: Some("text/plain".to_owned()),
            size: None,
        }]);

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["resources"][0]["uri"], "file:///ola.txt");
        assert_eq!(value["resources"][0]["mimeType"], "text/plain");
        assert!(value.get("nextCursor").is_none());
    }

    #[test]
    fn read_resource_result_serializes() {
        let result =
            ReadResourceResult::new(vec![ResourceContents::text("file:///ola.txt", "olá")]);

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(
            value["contents"][0],
            json!({ "uri": "file:///ola.txt", "text": "olá" })
        );
    }

    #[test]
    fn read_resource_params_round_trips() {
        let params = ReadResourceRequestParams {
            uri: "file:///x.txt".to_owned(),
        };

        let json = serde_json::to_string(&params).unwrap();
        let decoded: ReadResourceRequestParams = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, params);
    }

    #[test]
    fn resource_templates_list_result_serializes() {
        let result = ListResourceTemplatesResult::new(vec![ResourceTemplateMetadata {
            uri_template: "file:///{path}".to_owned(),
            name: "Arquivo".to_owned(),
            description: Some("Qualquer arquivo local".to_owned()),
            mime_type: None,
        }]);

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(
            value["resourceTemplates"][0]["uriTemplate"],
            "file:///{path}"
        );
        assert_eq!(value["resourceTemplates"][0]["name"], "Arquivo");
        assert!(value.get("nextCursor").is_none());
    }

    #[test]
    fn resource_templates_params_round_trips() {
        let params = ListResourceTemplatesParams {
            cursor: Some("p2".to_owned()),
        };

        let json = serde_json::to_string(&params).unwrap();
        let decoded: ListResourceTemplatesParams = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, params);
    }
}
