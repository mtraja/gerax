//! Erros da crate.
//!
//! [`McpError`] agrega erros de protocolo, aplicação e transporte em
//! um único tipo, mantendo a possibilidade de conversão direta para
//! um [`JsonRpcError`].
//!
//! Nunca retornar stack traces, credenciais ou dados internos ao
//! cliente MCP.

use crate::prompt::PromptError;
use crate::protocol::JsonRpcError;
use crate::resource::ResourceError;
use crate::tool::ToolError;
use crate::transport::TransportError;

/// Error central do `gerax-mcp`.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// Requisição malformada.
    #[error("requisição inválida: {0}")]
    InvalidRequest(String),
    /// Parâmetros inválidos.
    #[error("parâmetros inválidos: {0}")]
    InvalidParams(String),
    /// Método não suportado.
    #[error("método não encontrado: {0}")]
    MethodNotFound(String),
    /// Operação realizada antes de `initialize`/`initialized`.
    #[error("servidor não inicializado")]
    ServerNotInitialized,
    /// Tool inexistente.
    #[error("tool não encontrada: `{0}`")]
    ToolNotFound(String),
    /// Resource inexistente.
    #[error("resource não encontrado: `{0}`")]
    ResourceNotFound(String),
    /// Prompt inexistente.
    #[error("prompt não encontrado: `{0}`")]
    PromptNotFound(String),
    /// Operação negada pelo autorizador.
    #[error("acesso negado: {0}")]
    Authorization(String),
    /// Erro interno.
    #[error("erro interno: {0}")]
    Internal(String),
}

impl McpError {
    /// Converte para um [`JsonRpcError`], aplicando os códigos da
    /// especificação MCP.
    ///
    /// Códigos usados:
    /// * `-32700` parse; `-32600` invalid request; `-32601` method not
    ///   found; `-32602` invalid params (tool/prompt inexistente ou
    ///   argumentos inválidos); `-32603` erro interno;
    /// * `-32000` servidor não inicializado;
    /// * `-32001` não autorizado;
    /// * `-32002` resource não encontrado.
    pub fn to_jsonrpc_error(&self) -> JsonRpcError {
        match self {
            Self::InvalidRequest(message) => {
                JsonRpcError::new(JsonRpcError::INVALID_REQUEST, message.clone())
            }
            Self::InvalidParams(message) => {
                JsonRpcError::new(JsonRpcError::INVALID_PARAMS, message.clone())
            }
            Self::MethodNotFound(method) => JsonRpcError::new(
                JsonRpcError::METHOD_NOT_FOUND,
                format!("Method not found: {method}"),
            ),
            Self::ServerNotInitialized => JsonRpcError::new(-32000, "Server not initialized"),
            Self::ToolNotFound(name) => JsonRpcError::new(
                JsonRpcError::INVALID_PARAMS,
                format!("Unknown tool: {name}"),
            ),
            Self::ResourceNotFound(uri) => {
                JsonRpcError::new(-32002, format!("Resource not found: {uri}"))
            }
            Self::PromptNotFound(name) => JsonRpcError::new(
                JsonRpcError::INVALID_PARAMS,
                format!("Unknown prompt: {name}"),
            ),
            Self::Authorization(reason) => {
                JsonRpcError::new(-32001, format!("Not authorized: {reason}"))
            }
            Self::Internal(message) => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, message.clone())
            }
        }
    }
}

impl From<serde_json::Error> for McpError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidParams(error.to_string())
    }
}

impl From<ToolError> for McpError {
    fn from(error: ToolError) -> Self {
        match error {
            ToolError::Deserialize(message) => Self::InvalidParams(message.to_string()),
            ToolError::InvalidArgs { name, reason } => Self::InvalidParams(format!(
                "argumentos inválidos para a tool `{name}`: {reason}"
            )),
            ToolError::Internal(message) => Self::Internal(message),
            ToolError::InvalidTool(name) => Self::InvalidParams(format!("tool inválida: {name}")),
            ToolError::Duplicate { .. } => Self::Internal("tool duplicada".to_owned()),
            ToolError::NotFound { name } => Self::ToolNotFound(name),
        }
    }
}

impl From<ResourceError> for McpError {
    fn from(error: ResourceError) -> Self {
        match error {
            ResourceError::NotFound(uri) => Self::ResourceNotFound(uri),
            ResourceError::Duplicate(..) | ResourceError::InvalidResource(_) => {
                Self::Internal("resource inválido".to_owned())
            }
            ResourceError::Read(_, _) | ResourceError::Internal(_) => {
                Self::Internal(error.to_string())
            }
        }
    }
}

impl From<PromptError> for McpError {
    fn from(error: PromptError) -> Self {
        match error {
            PromptError::Deserialize(message) => Self::InvalidParams(message.to_string()),
            PromptError::InvalidArgs { name, reason } => Self::InvalidParams(format!(
                "argumentos inválidos para o prompt `{name}`: {reason}"
            )),
            PromptError::Internal(message) => Self::Internal(message),
            PromptError::InvalidPrompt(name) => {
                Self::InvalidParams(format!("prompt inválido: {name}"))
            }
            PromptError::Duplicate { .. } => Self::Internal("prompt duplicado".to_owned()),
            PromptError::NotFound { name } => Self::PromptNotFound(name),
        }
    }
}

impl From<TransportError> for McpError {
    fn from(error: TransportError) -> Self {
        Self::Internal(error.to_string())
    }
}

use crate::security::AuthorizationError;

impl From<AuthorizationError> for McpError {
    fn from(error: AuthorizationError) -> Self {
        match error {
            AuthorizationError::Denied(reason) => Self::Authorization(reason),
            AuthorizationError::Internal(reason) => Self::Internal(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_invalid_request() {
        let error = McpError::InvalidRequest("x".into());
        let jsonrpc = error.to_jsonrpc_error();

        assert_eq!(jsonrpc.code, -32600);
    }

    #[test]
    fn maps_unknown_tool_to_invalid_params() {
        let error = McpError::ToolNotFound("echo".into());
        let jsonrpc = error.to_jsonrpc_error();

        assert_eq!(jsonrpc.code, -32602);
        assert_eq!(jsonrpc.message, "Unknown tool: echo");
    }

    #[test]
    fn maps_resource_not_found() {
        let error = McpError::ResourceNotFound("file:///x".into());
        let jsonrpc = error.to_jsonrpc_error();

        assert_eq!(jsonrpc.code, -32002);
    }

    #[test]
    fn maps_server_not_initialized() {
        let error = McpError::ServerNotInitialized;
        let jsonrpc = error.to_jsonrpc_error();

        assert_eq!(jsonrpc.code, -32000);
    }

    #[test]
    fn maps_internal_error() {
        let error = McpError::Internal("boom".into());
        let jsonrpc = error.to_jsonrpc_error();

        assert_eq!(jsonrpc.code, -32603);
    }

    #[test]
    fn converts_tool_not_found() {
        let error = McpError::from(ToolError::NotFound {
            name: "echo".into(),
        });

        assert!(matches!(error, McpError::ToolNotFound(name) if name == "echo"));
    }

    #[test]
    fn converts_authorization_denied() {
        let error = McpError::from(AuthorizationError::Denied("sem permissão".into()));

        assert!(matches!(error, McpError::Authorization(_)));
    }
}
