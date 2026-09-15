//! Definição da abstração [`Tool`] e de seus erros.

use async_trait::async_trait;
use serde_json::Value;

use super::invocation::CallToolResult;

/// Erro retornado durante registro ou invocação de uma Tool.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Falha ao desserializar os argumentos da Tool.
    #[error("falha ao desserializar os argumentos da tool: {0}")]
    Deserialize(#[from] serde_json::Error),
    /// A Tool recebeu argumentos inválidos.
    #[error("argumentos inválidos para a tool `{name}`: {reason}")]
    InvalidArgs {
        /// Nome da Tool.
        name: String,
        /// Motivo da rejeição dos argumentos.
        reason: String,
    },
    /// Erro interno da implementação da Tool.
    #[error("erro interno da tool: {0}")]
    Internal(String),
    /// Tentativa de registrar uma Tool com nome inválido.
    #[error("tool inválida: `{0}`")]
    InvalidTool(String),
    /// Tentativa de registrar uma Tool com nome já existente.
    #[error("a tool `{name}` já está registrada")]
    Duplicate {
        /// Nome duplicado.
        name: String,
    },
    /// Tentativa de invocar uma Tool inexistente.
    #[error("a tool `{name}` não existe")]
    NotFound {
        /// Nome da Tool inexistente.
        name: String,
    },
}

/// Uma Tool invocável exposta pelo servidor MCP.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Nome único da Tool.
    fn name(&self) -> &str;

    /// Descrição da Tool, quando disponível.
    fn description(&self) -> Option<&str> {
        None
    }

    /// Schema JSON de entrada da Tool.
    fn input_schema(&self) -> Value;

    /// Executa a Tool com os argumentos fornecidos.
    async fn call(&self, arguments: Value) -> Result<CallToolResult, ToolError>;
}
