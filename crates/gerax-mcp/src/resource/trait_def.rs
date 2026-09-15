//! Traits e tipos do modelo de Resources.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Conteúdo retornado por um resource: texto ou binário.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContents {
    /// Conteúdo textual.
    Text(TextResourceContents),
    /// Conteúdo binário (Base64) ou BLOB.
    Blob(BlobResourceContents),
}

impl ResourceContents {
    /// Cria um conteúdo textual.
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Text(TextResourceContents {
            uri: uri.into(),
            mime_type: None,
            text: text.into(),
        })
    }

    /// Cria um conteúdo textual com MIME type.
    pub fn text_with_mime(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        let mut contents = Self::text(uri, text);
        if let Self::Text(text) = &mut contents {
            text.mime_type = Some(mime_type.into());
        }
        contents
    }

    /// Cria um conteúdo binário (Base64).
    pub fn blob(uri: impl Into<String>, blob: impl Into<String>) -> Self {
        Self::Blob(BlobResourceContents {
            uri: uri.into(),
            mime_type: None,
            blob: blob.into(),
        })
    }

    /// Cria um conteúdo binário (Base64) com MIME type.
    pub fn blob_with_mime(
        uri: impl Into<String>,
        mime_type: impl Into<String>,
        blob: impl Into<String>,
    ) -> Self {
        let mut contents = Self::blob(uri, blob);
        if let Self::Blob(blob) = &mut contents {
            blob.mime_type = Some(mime_type.into());
        }
        contents
    }

    /// URI do resource.
    pub fn uri(&self) -> &str {
        match self {
            Self::Text(contents) => &contents.uri,
            Self::Blob(contents) => &contents.uri,
        }
    }

    /// MIME type do conteúdo, se houver.
    pub fn mime_type(&self) -> Option<&str> {
        match self {
            Self::Text(contents) => contents.mime_type.as_deref(),
            Self::Blob(contents) => contents.mime_type.as_deref(),
        }
    }

    /// Tamanho em bytes quando conhecido.
    pub fn size(&self) -> Option<u64> {
        match self {
            Self::Text(contents) => Some(contents.text.len() as u64),
            Self::Blob(contents) => Some(contents.blob.len() as u64),
        }
    }
}

/// Conteúdo textual de um resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextResourceContents {
    /// URI do resource.
    pub uri: String,
    /// MIME type do conteúdo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Conteúdo textual.
    pub text: String,
}

/// Conteúdo binário (Base64) de um resource.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobResourceContents {
    /// URI do resource.
    pub uri: String,
    /// MIME type do conteúdo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Conteúdo binário codificado em Base64.
    pub blob: String,
}

/// Erros de Resource.
///
/// Um resource legível pode falhar com:
/// * Resource não encontrado: `-32002` (JSON-RPC),
/// * Erros internos: `-32603` (JSON-RPC),
/// * Duplicação na hora de registrar.
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    /// Resource não encontrado pelo URI.
    #[error("resource não encontrado: `{0}`")]
    NotFound(String),
    /// Resource com URI duplicado.
    #[error("já existe um resource com o URI `{0}`")]
    Duplicate(String),
    /// Resource inválido (URI vazio).
    #[error("resource inválido: `{0}`")]
    InvalidResource(String),
    /// Erro ao ler o resource.
    #[error("falha ao ler o resource `{0}`: {1}")]
    Read(String, String),
    /// Erro interno do resource.
    #[error("erro interno: {0}")]
    Internal(String),
}

/// Um recurso legível exposto pelo servidor MCP.
///
/// Cada resource é identificado unicamente por uma URI e produz
/// [`ResourceContents`] textual ou binário.
#[async_trait]
pub trait Resource: Send + Sync {
    /// URI do resource.
    fn uri(&self) -> &str;
    /// Nome do resource (exibível ao cliente).
    fn name(&self) -> &str;
    /// Descrição opcional.
    fn description(&self) -> Option<&str> {
        None
    }
    /// MIME type do recurso.
    fn mime_type(&self) -> Option<&str> {
        None
    }
    /// Conteúdo do recurso.
    ///
    /// # Erros
    ///
    /// Retorna [`ResourceError`] se o resource não puder ser lido.
    async fn read(&self) -> Result<ResourceContents, ResourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn text_contents_serializes_with_uri_and_text() {
        let contents =
            ResourceContents::text_with_mime("file:///ola.txt", "text/plain", "Olá mundo");

        assert_eq!(
            serde_json::to_value(&contents).unwrap(),
            json!({ "uri": "file:///ola.txt", "mimeType": "text/plain", "text": "Olá mundo" })
        );
    }

    #[test]
    fn blob_contents_serializes_with_uri_and_blob() {
        let contents = ResourceContents::blob_with_mime("file:///img.png", "image/png", "aGVsbG8=");

        assert_eq!(
            serde_json::to_value(&contents).unwrap(),
            json!({ "uri": "file:///img.png", "mimeType": "image/png", "blob": "aGVsbG8=" })
        );
    }

    #[test]
    fn text_contents_deserializes_roundtrip() {
        let value = json!({ "uri": "file:///ola.txt", "text": "Olá" });
        let contents: ResourceContents = serde_json::from_value(value).unwrap();

        assert_eq!(contents, ResourceContents::text("file:///ola.txt", "Olá"));
    }

    #[test]
    fn helpers_expose_metadata() {
        let text = ResourceContents::text("file:///t.txt", "hey");
        let blob = ResourceContents::blob_with_mime("file:///b.png", "image/png", "AQID");

        assert_eq!(text.uri(), "file:///t.txt");
        assert_eq!(text.mime_type(), None);
        assert_eq!(text.size(), Some(3));
        assert_eq!(blob.uri(), "file:///b.png");
        assert_eq!(blob.mime_type(), Some("image/png"));
        assert_eq!(blob.size(), Some(4));
    }
}
