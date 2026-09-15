//! Erros do cliente MCP.
//!
//! [`McpClientError`] cobre os estados do ciclo de vida do cliente,
//! falhas de transporte, respostas malformadas e erros reportados
//! pelo servidor. Nunca inclui secrets, credenciais ou argumentos
//! sensíveis.

use serde_json::Value;

use crate::protocol::JsonRpcId;
use crate::transport::TransportError;

/// Erro central do cliente MCP.
#[derive(Debug, thiserror::Error)]
pub enum McpClientError {
    /// Operação executada antes de `initialize` concluir o handshake.
    #[error("cliente não inicializado: chame `initialize()` antes desta operação")]
    NotInitialized,
    /// Tentativa de inicializar um cliente já inicializado.
    #[error("cliente já inicializado")]
    AlreadyInitialized,
    /// Operação executada após o fechamento do cliente.
    #[error("cliente fechado")]
    Closed,
    /// O request não recebeu resposta dentro do prazo configurado.
    #[error("request expirou: nenhuma resposta dentro do timeout")]
    Timeout,
    /// Falha no transporte subjacente.
    #[error("erro de transporte: {0}")]
    Transport(#[from] TransportError),
    /// Falha ao desserializar o resultado do servidor.
    #[error("falha ao interpretar a resposta do servidor: {0}")]
    Deserialize(#[from] serde_json::Error),
    /// O servidor respondeu com um erro JSON-RPC.
    #[error(
        "erro do servidor MCP em `{method}` (id {id}): {code} {message}{data}",
        data = .data.as_ref().map(|v| format!(" — dados: {v:?}")).unwrap_or_default()
    )]
    Server {
        /// Método JSON-RPC do request.
        method: String,
        /// ID do request.
        id: JsonRpcId,
        /// Código do erro JSON-RPC.
        code: i32,
        /// Mensagem do erro.
        message: String,
        /// Dados adicionais do erro, quando houver.
        data: Option<Value>,
    },
    /// A resposta não possui `result` nem `error` utilizáveis.
    #[error("resposta inválida para `{method}` (id {id}): {reason}")]
    InvalidResponse {
        /// Método JSON-RPC do request.
        method: String,
        /// ID do request.
        id: JsonRpcId,
        /// Motivo da rejeição.
        reason: String,
    },
    /// Configuração inválida fornecida ao builder.
    #[error("configuração inválida do cliente: {0}")]
    InvalidConfig(String),
    /// Operação rejeitada porque o servidor não anuncia a capability
    /// necessária (`tools`, `resources`, `prompts`, ...).
    #[error("operação não suportada pelo servidor: {0}")]
    Unsupported(String),
}

impl McpClientError {
    pub(crate) fn invalid_config(reason: impl Into<String>) -> Self {
        Self::InvalidConfig(reason.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn server_error_recovers_diagnostic_fields() {
        let error = McpClientError::Server {
            method: "tools/call".to_owned(),
            id: JsonRpcId::Number(42),
            code: -32602,
            message: "Invalid arguments".to_owned(),
            data: None,
        };

        let rendered = error.to_string();

        assert!(rendered.contains("tools/call"));
        assert!(rendered.contains("42"));
        assert!(rendered.contains("-32602"));
        assert!(rendered.contains("Invalid arguments"));
    }

    #[test]
    fn server_error_appends_data_debug() {
        let error = McpClientError::Server {
            method: "tools/call".to_owned(),
            id: JsonRpcId::Number(1),
            code: -32603,
            message: "boom".to_owned(),
            data: Some(json!({ "detail": "falhou" })),
        };

        let rendered = error.to_string();

        assert!(rendered.contains("dados"));
        assert!(rendered.contains("falhou"));
    }
}
