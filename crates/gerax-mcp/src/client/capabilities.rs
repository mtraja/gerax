//! Conveniências de capabilities do cliente MCP.
//!
//! Mantém consultas comuns sobre as capabilities anunciadas pelo
//! servidor, evitando acessar a estrutura [`ServerCapabilities`]
//! diretamente na lógica de negócio do cliente.

use crate::protocol::ServerCapabilities;

use super::session::McpSession;

impl McpSession {
    /// Verifica se o servidor anuncia suporte a Tools.
    pub fn supports_tools(&self) -> bool {
        self.server_capabilities.tools.is_some()
    }

    /// Verifica se o servidor anuncia suporte a Resources.
    pub fn supports_resources(&self) -> bool {
        self.server_capabilities.resources.is_some()
    }

    /// Verifica se o servidor anuncia suporte a Prompts.
    pub fn supports_prompts(&self) -> bool {
        self.server_capabilities.prompts.is_some()
    }

    /// Verifica se o servidor anuncia suporte a Completions.
    pub fn supports_completions(&self) -> bool {
        self.server_capabilities.completions.is_some()
    }

    /// Verifica se o servidor anuncia suporte a logs estruturados.
    pub fn supports_logging(&self) -> bool {
        self.server_capabilities.logging.is_some()
    }
}

/// Deriva as capabilities anunciadas a partir da presença de capacidades.
impl From<&ServerCapabilities> for Vec<&'static str> {
    fn from(capabilities: &ServerCapabilities) -> Self {
        let mut present = Vec::new();
        if capabilities.tools.is_some() {
            present.push("tools");
        }
        if capabilities.resources.is_some() {
            present.push("resources");
        }
        if capabilities.prompts.is_some() {
            present.push("prompts");
        }
        if capabilities.completions.is_some() {
            present.push("completions");
        }
        if capabilities.logging.is_some() {
            present.push("logging");
        }
        present
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        ImplementationInfo, ServerPromptsCapability, ServerResourcesCapability,
        ServerToolsCapability,
    };

    fn session(capabilities: ServerCapabilities) -> McpSession {
        McpSession::new(
            "2025-11-25",
            ImplementationInfo::new("srv", "1"),
            capabilities,
        )
    }

    #[test]
    fn session_reports_supported_capabilities() {
        let session = session(
            ServerCapabilities::default()
                .with_tools()
                .with_resources()
                .with_prompts(),
        );

        assert!(session.supports_tools());
        assert!(session.supports_resources());
        assert!(session.supports_prompts());
        assert!(!session.supports_completions());
        assert!(!session.supports_logging());
    }

    #[test]
    fn empty_capabilities_reports_nothing() {
        let session = session(ServerCapabilities::default());

        assert!(!session.supports_tools());
        assert!(!session.supports_resources());
        assert!(!session.supports_prompts());
    }

    #[test]
    fn flags_capabilities_only_when_declared() {
        let capabilities = ServerCapabilities {
            tools: Some(ServerToolsCapability::default()),
            resources: Some(ServerResourcesCapability::default()),
            prompts: Some(ServerPromptsCapability::default()),
            ..Default::default()
        };

        let declared: Vec<&str> = Vec::from(&capabilities);

        assert_eq!(declared, vec!["tools", "resources", "prompts"]);
    }
}
