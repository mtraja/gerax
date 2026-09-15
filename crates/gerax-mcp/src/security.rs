//! Pontos de extensão de segurança.
//!
//! `gerax-mcp` não implementa autenticação própria. Em vez disso,
//! expõe hooks de autorização que podem ser conectados ao servidor:
//!
//! ```text
//! Request
//!    ↓
//! Authentication (externa)
//!    ↓
//! Authorization (McpAuthorizer)
//!    ↓
//! Dispatcher
//! ```

use async_trait::async_trait;

use crate::context::McpContext;

/// Operação MCP que pode ser auditada/negada por um autorizador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpOperation {
    /// `initialize`.
    Initialize,
    /// `notifications/initialized`.
    NotificationsInitialized,
    /// `tools/list`.
    ListTools,
    /// `tools/call`.
    CallTool(String),
    /// `resources/list`.
    ListResources,
    /// `resources/read`.
    ReadResource(String),
    /// `prompts/list`.
    ListPrompts,
    /// `prompts/get`.
    GetPrompt(String),
}

impl std::fmt::Display for McpOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initialize => f.write_str("initialize"),
            Self::NotificationsInitialized => f.write_str("notifications/initialized"),
            Self::ListTools => f.write_str("tools/list"),
            Self::CallTool(name) => write!(f, "tools/call({name})"),
            Self::ListResources => f.write_str("resources/list"),
            Self::ReadResource(uri) => write!(f, "resources/read({uri})"),
            Self::ListPrompts => f.write_str("prompts/list"),
            Self::GetPrompt(name) => write!(f, "prompts/get({name})"),
        }
    }
}

/// Erro de autorização.
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    /// Operação negada.
    #[error("acesso negado: {0}")]
    Denied(String),
    /// Falha interna durante a avaliação.
    #[error("falha interna de autorização: {0}")]
    Internal(String),
}

/// Autorizador plugável do servidor.
///
/// Implemente para controlar **quem** pode executar cada operação. A
/// autenticação em si deve ocorrer externamente (transporte/proxy).
#[async_trait]
pub trait McpAuthorizer: Send + Sync {
    /// Autoriza ou nega a operação.
    ///
    /// Devolve `Ok(())` para permitir ou [`AuthorizationError::Denied`]
    /// para negar.
    async fn authorize(
        &self,
        context: &McpContext,
        operation: &McpOperation,
    ) -> Result<(), AuthorizationError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operations_format() {
        assert_eq!(
            McpOperation::CallTool("echo".into()).to_string(),
            "tools/call(echo)"
        );
        assert_eq!(McpOperation::Initialize.to_string(), "initialize");
        assert_eq!(
            McpOperation::ReadResource("file:///x".into()).to_string(),
            "resources/read(file:///x)"
        );
    }

    #[async_trait]
    impl McpAuthorizer for () {
        async fn authorize(
            &self,
            _context: &McpContext,
            _operation: &McpOperation,
        ) -> Result<(), AuthorizationError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn authorizer_can_be_a_noop() {
        let context = McpContext::new(crate::protocol::JsonRpcId::number(1), "tools/list");

        ().authorize(&context, &McpOperation::ListTools)
            .await
            .unwrap();
    }
}
