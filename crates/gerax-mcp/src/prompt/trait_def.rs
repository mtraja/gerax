//! Traits e tipos do modelo de Prompts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Papel do emissor de uma mensagem de prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptRole {
    /// Mensagem do usuário.
    User,
    /// Mensagem do assistente (contexto/gold).
    Assistant,
}

/// Conteúdo de uma mensagem de prompt.
///
/// Segue o wire format MCP: blocos com `type` `"text"` ou `"image"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptContent {
    /// Bloco textual.
    Text(PromptTextContent),
    /// Bloco de imagem (base64).
    Image(PromptImageContent),
}

impl PromptContent {
    /// Cria um bloco textual.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(PromptTextContent::new(text))
    }
}

impl From<&str> for PromptContent {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

impl From<String> for PromptContent {
    fn from(text: String) -> Self {
        Self::text(text)
    }
}

/// Bloco de conteúdo textual de um prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTextContent {
    /// Texto do bloco.
    pub text: String,
}

impl PromptTextContent {
    /// Cria um bloco textual.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// Bloco de conteúdo de imagem (base64) de um prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptImageContent {
    /// Dados da imagem em base64.
    pub data: String,
    /// Tipo MIME da imagem.
    pub mime_type: String,
}

impl PromptImageContent {
    /// Cria um bloco de imagem.
    pub fn new(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }
}

/// Mensagem de um prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMessage {
    /// Papel do emissor.
    pub role: PromptRole,
    /// Conteúdo da mensagem.
    pub content: PromptContent,
}

impl PromptMessage {
    /// Cria uma mensagem.
    pub fn new(role: PromptRole, content: PromptContent) -> Self {
        Self { role, content }
    }

    /// Cria uma mensagem `user`.
    pub fn user(content: PromptContent) -> Self {
        Self::new(PromptRole::User, content)
    }

    /// Cria uma mensagem `assistant`.
    pub fn assistant(content: PromptContent) -> Self {
        Self::new(PromptRole::Assistant, content)
    }
}

/// Resultado de `prompts/get`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPromptResult {
    /// Descrição opcional do prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Mensagens do prompt.
    pub messages: Vec<PromptMessage>,
}

impl GetPromptResult {
    /// Cria um resultado de prompt.
    pub fn new(description: Option<String>, messages: Vec<PromptMessage>) -> Self {
        Self {
            description,
            messages,
        }
    }
}

/// Argumento declarado por um prompt (`prompts/list`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptArgument {
    /// Nome do argumento.
    pub name: String,
    /// Descrição do argumento, opcional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Se o argumento é obrigatório.
    #[serde(skip_serializing_if = "is_false")]
    pub required: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl PromptArgument {
    /// Cria um argumento.
    pub fn new(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        required: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.map(Into::into),
            required,
        }
    }
}

/// Erros de registro ou obtenção de Prompts.
#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    /// Falha ao desserializar os argumentos do prompt.
    #[error("falha ao desserializar os argumentos do prompt: {0}")]
    Deserialize(#[from] serde_json::Error),
    /// O prompt recebeu argumentos inválidos.
    #[error("argumentos inválidos para o prompt `{name}`: {reason}")]
    InvalidArgs {
        /// Nome do prompt.
        name: String,
        /// Motivo da rejeição dos argumentos.
        reason: String,
    },
    /// Erro interno da implementação do prompt.
    #[error("erro interno do prompt: {0}")]
    Internal(String),
    /// Tentativa de registrar um prompt com nome inválido.
    #[error("prompt inválido: `{0}`")]
    InvalidPrompt(String),
    /// Tentativa de registrar um prompt com nome já existente.
    #[error("o prompt `{name}` já está registrado")]
    Duplicate {
        /// Nome duplicado.
        name: String,
    },
    /// Tentativa de obter um prompt inexistente.
    #[error("o prompt `{name}` não existe")]
    NotFound {
        /// Nome do prompt inexistente.
        name: String,
    },
}

/// Um prompt exposto pelo servidor MCP.
#[async_trait]
pub trait Prompt: Send + Sync {
    /// Nome único do prompt.
    fn name(&self) -> &str;

    /// Descrição do prompt, quando disponível.
    fn description(&self) -> Option<&str> {
        None
    }

    /// Argumentos declarados do prompt.
    fn arguments(&self) -> Vec<PromptArgument> {
        Vec::new()
    }

    /// Obtém as mensagens do prompt com os argumentos fornecidos.
    async fn get(&self, arguments: Value) -> Result<GetPromptResult, PromptError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_message_serializes() {
        let message = PromptMessage::user(PromptContent::text("olá"));

        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            json!({
                "role": "user",
                "content": { "type": "text", "text": "olá" }
            })
        );
    }

    #[test]
    fn image_content_serializes() {
        let content = PromptContent::Image(PromptImageContent::new("aGVsbG8=", "image/png"));

        assert_eq!(
            serde_json::to_value(&content).unwrap(),
            json!({ "type": "image", "data": "aGVsbG8=", "mimeType": "image/png" })
        );
    }

    #[test]
    fn get_prompt_result_serializes() {
        let result = GetPromptResult::new(
            Some("Code review".into()),
            vec![PromptMessage::assistant(PromptContent::text("ok"))],
        );

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["description"], "Code review");
        assert_eq!(value["messages"][0]["role"], "assistant");
        assert_eq!(value["messages"][0]["content"]["type"], "text");
    }

    #[test]
    fn get_prompt_result_without_description_omits_it() {
        let result = GetPromptResult::new(None, vec![]);

        let value = serde_json::to_value(&result).unwrap();

        assert!(value.get("description").is_none());
        assert_eq!(value["messages"], json!([]));
    }

    #[test]
    fn prompt_argument_serializes() {
        let argument = PromptArgument::new("code", Some("The code"), true);

        assert_eq!(
            serde_json::to_value(&argument).unwrap(),
            json!({ "name": "code", "description": "The code", "required": true })
        );
    }

    #[test]
    fn prompt_argument_omits_optional_fields() {
        let argument = PromptArgument::new("code", None::<String>, false);

        assert_eq!(
            serde_json::to_value(&argument).unwrap(),
            json!({ "name": "code" })
        );
    }

    #[test]
    fn prompt_message_round_trips() {
        let message = PromptMessage::assistant(PromptContent::text("ok"));

        let json = serde_json::to_string(&message).unwrap();
        let decoded: PromptMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, message);
    }
}
