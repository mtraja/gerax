//! Contexto de execução de um request MCP.
//!
//! [`McpContext`] expõe metadados do request (id, método, cliente e
//! versão do protocolo) para autorização e extensões, sem depender de
//! framework HTTP.

use crate::protocol::{ImplementationInfo, JsonRpcId};

/// Metadados de um request MCP sendo processado.
#[derive(Debug, Clone)]
pub struct McpContext {
    /// Identificador do request JSON-RPC.
    pub request_id: JsonRpcId,
    /// Método JSON-RPC sendo invocado.
    pub method: String,
    /// Informações do cliente, após `initialize`.
    pub client_info: Option<ImplementationInfo>,
    /// Versão do protocolo negociada.
    pub protocol_version: Option<String>,
}

impl McpContext {
    /// Cria um contexto a partir de um request.
    pub fn new(request_id: JsonRpcId, method: impl Into<String>) -> Self {
        Self {
            request_id,
            method: method.into(),
            client_info: None,
            protocol_version: None,
        }
    }

    /// Define as informações do cliente.
    pub fn with_client_info(mut self, client_info: Option<ImplementationInfo>) -> Self {
        self.client_info = client_info;
        self
    }

    /// Define a versão de protocolo negociada.
    pub fn with_protocol_version(mut self, protocol_version: Option<String>) -> Self {
        self.protocol_version = protocol_version;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_captures_request_metadata() {
        let context = McpContext::new(JsonRpcId::number(1), "tools/list")
            .with_client_info(Some(ImplementationInfo::new("c", "1")))
            .with_protocol_version(Some("2025-11-25".to_owned()));

        assert_eq!(context.request_id, JsonRpcId::number(1));
        assert_eq!(context.method, "tools/list");
        assert!(context.client_info.is_some());
        assert_eq!(context.protocol_version.as_deref(), Some("2025-11-25"));
    }
}
