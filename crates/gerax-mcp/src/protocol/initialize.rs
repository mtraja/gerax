//! Mensagens de inicialização do MCP.
//!
//! Implementa o método `initialize`, responsável pela negociação de
//! versão de protocolo, troca de capabilities e identificação das
//! implementações, conforme o lifecycle da especificação vigente.

use serde::{Deserialize, Serialize};

use super::capabilities::{ClientCapabilities, ServerCapabilities};

/// Método JSON-RPC `initialize`.
pub const INITIALIZE: &str = "initialize";

/// Versão do protocolo MCP suportada por esta crate.
///
/// Conforme a especificação vigente adotada pelo Gerax.
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

/// Identificação de uma implementação MCP (cliente ou servidor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImplementationInfo {
    /// Nome da implementação.
    pub name: String,
    /// Versão da implementação.
    pub version: String,
}

impl ImplementationInfo {
    /// Cria informações de implementação.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// Parâmetros do request `initialize`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestParams {
    /// Versão mais recente do protocolo MCP suportada pelo cliente.
    pub protocol_version: String,
    /// Capabilities declaradas pelo cliente.
    pub capabilities: ClientCapabilities,
    /// Identificação da implementação do cliente.
    pub client_info: ImplementationInfo,
}

impl InitializeRequestParams {
    /// Cria os parâmetros de um request `initialize`.
    pub fn new(
        protocol_version: impl Into<String>,
        capabilities: ClientCapabilities,
        client_info: ImplementationInfo,
    ) -> Self {
        Self {
            protocol_version: protocol_version.into(),
            capabilities,
            client_info,
        }
    }
}

/// Resultado do request `initialize`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Versão do protocolo MCP escolhida pelo servidor.
    pub protocol_version: String,
    /// Capabilities implementadas pelo servidor.
    pub capabilities: ServerCapabilities,
    /// Identificação da implementação do servidor.
    pub server_info: ImplementationInfo,
    /// Instruções de uso do servidor, opcionais.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

impl InitializeResult {
    /// Cria um resultado de `initialize`.
    pub fn new(
        protocol_version: impl Into<String>,
        capabilities: ServerCapabilities,
        server_info: ImplementationInfo,
    ) -> Self {
        Self {
            protocol_version: protocol_version.into(),
            capabilities,
            server_info,
            instructions: None,
        }
    }

    /// Define as instruções de uso do servidor.
    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn client() -> ClientCapabilities {
        ClientCapabilities::default().with_roots()
    }

    #[test]
    fn implementation_info_serializes() {
        let info = ImplementationInfo::new("my-client", "1.0.0");

        let value = serde_json::to_value(&info).unwrap();

        assert_eq!(value, json!({ "name": "my-client", "version": "1.0.0" }));
    }

    #[test]
    fn initialize_request_params_serializes_camel_case() {
        let params = InitializeRequestParams::new(
            MCP_PROTOCOL_VERSION,
            client(),
            ImplementationInfo::new("my-client", "1.0.0"),
        );

        let value = serde_json::to_value(&params).unwrap();

        assert_eq!(value["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(value["capabilities"], json!({ "roots": {} }));
        assert_eq!(value["clientInfo"]["name"], "my-client");
    }

    #[test]
    fn initialize_result_serializes() {
        let result = InitializeResult::new(
            MCP_PROTOCOL_VERSION,
            ServerCapabilities::default().with_tools(),
            ImplementationInfo::new("escola-cqrs", "0.1.0"),
        );

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(value["capabilities"], json!({ "tools": {} }));
        assert_eq!(value["serverInfo"]["name"], "escola-cqrs");
        assert!(value.get("instructions").is_none());
    }

    #[test]
    fn initialize_result_with_instructions() {
        let result = InitializeResult::new(
            MCP_PROTOCOL_VERSION,
            ServerCapabilities::default(),
            ImplementationInfo::new("srv", "1"),
        )
        .with_instructions("use with care");

        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["instructions"], "use with care");
    }

    #[test]
    fn initialize_request_params_round_trips() {
        let params = InitializeRequestParams::new(
            MCP_PROTOCOL_VERSION,
            client(),
            ImplementationInfo::new("my-client", "1.0.0"),
        );

        let json = serde_json::to_string(&params).unwrap();
        let decoded: InitializeRequestParams = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, params);
    }

    #[test]
    fn initialize_result_round_trips() {
        let result = InitializeResult::new(
            MCP_PROTOCOL_VERSION,
            ServerCapabilities::default().with_tools(),
            ImplementationInfo::new("srv", "1"),
        );

        let json = serde_json::to_string(&result).unwrap();
        let decoded: InitializeResult = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, result);
    }
}
