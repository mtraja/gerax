//! Sessão do cliente MCP.
//!
//! [`McpSession`] representa o estado negociado com o servidor após o
//! handshake `initialize`: versão de protocolo, identificação do
//! servidor e suas capabilities.

use crate::protocol::{ImplementationInfo, ServerCapabilities};

/// Identificação do servidor MCP, derivada do `initialize`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    /// Nome da implementação do servidor.
    pub name: String,
    /// Versão da implementação do servidor.
    pub version: String,
}

impl From<&ImplementationInfo> for ServerInfo {
    fn from(info: &ImplementationInfo) -> Self {
        Self {
            name: info.name.clone(),
            version: info.version.clone(),
        }
    }
}

/// Estado negociado com o servidor MCP.
///
/// ```text
/// Created → Connected → Initializing → Initialized → Closed
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct McpSession {
    /// Versão do protocolo MCP acordada na inicialização.
    pub protocol_version: String,
    /// Identificação do servidor (nome e versão).
    pub server_info: ImplementationInfo,
    /// Capabilities anunciadas pelo servidor.
    pub server_capabilities: ServerCapabilities,
}

impl McpSession {
    /// Cria uma sessão a partir do resultado do `initialize`.
    pub fn new(
        protocol_version: impl Into<String>,
        server_info: ImplementationInfo,
        server_capabilities: ServerCapabilities,
    ) -> Self {
        Self {
            protocol_version: protocol_version.into(),
            server_info,
            server_capabilities,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ServerCapabilities;

    #[test]
    fn session_exposes_negotiated_values() {
        let session = McpSession::new(
            "2025-11-25",
            ImplementationInfo::new("escola-cqrs", "0.1.0"),
            ServerCapabilities::default().with_tools(),
        );

        assert_eq!(session.protocol_version, "2025-11-25");
        assert_eq!(session.server_info.name, "escola-cqrs");
        assert_eq!(session.server_info.version, "0.1.0");
        assert!(session.server_capabilities.tools.is_some());
    }

    #[test]
    fn server_info_derives_from_implementation_info() {
        let info = ServerInfo::from(&ImplementationInfo::new("escola-cqrs", "0.1.0"));

        assert_eq!(info.name, "escola-cqrs");
        assert_eq!(info.version, "0.1.0");
    }
}
