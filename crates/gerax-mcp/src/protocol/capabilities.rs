//! Capabilities de cliente e servidor negociadas no `initialize`.
//!
//! Apenas capabilities efetivamente habilitadas são serializadas
//! (campos `None` são omitidos da mensagem).

use serde::{Deserialize, Serialize};
use serde_json::Value;

fn is_false(value: &bool) -> bool {
    !*value
}

/// Capabilities declaradas pelo cliente.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Suporte a *roots* (diretórios de trabalho do cliente).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roots: Option<ClientRootsCapability>,
    /// Suporte a sampling LLM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
    /// Suporte a elicitation (coleta de informações do usuário).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<Value>,
    /// Capabilities experimentais não padronizadas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

impl ClientCapabilities {
    /// Habilita a capability `roots`.
    pub fn with_roots(mut self) -> Self {
        self.roots = Some(ClientRootsCapability::default());
        self
    }

    /// Habilita a capability `sampling`.
    pub fn with_sampling(mut self) -> Self {
        self.sampling = Some(Value::Object(Default::default()));
        self
    }
}

/// Capacidade do cliente de fornecer *roots*.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRootsCapability {
    /// Cliente suporta notificações de mudança de roots.
    #[serde(default, skip_serializing_if = "is_false")]
    pub list_changed: bool,
}

/// Capabilities declaradas pelo servidor.
///
/// A resposta de `initialize` deve refletir somente as capabilities
/// efetivamente implementadas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Capabilities experimentais não padronizadas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
    /// Suporte a logs estruturados.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
    /// O servidor oferece prompt templates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts: Option<ServerPromptsCapability>,
    /// O servidor fornece resources legíveis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ServerResourcesCapability>,
    /// O servidor expõe tools invocáveis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<ServerToolsCapability>,
    /// Suporte a autocompletion de argumentos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completions: Option<Value>,
}

impl ServerCapabilities {
    /// Habilita a capability `logging`.
    pub fn with_logging(mut self) -> Self {
        self.logging = Some(Value::Object(Default::default()));
        self
    }

    /// Habilita a capability `prompts`.
    pub fn with_prompts(mut self) -> Self {
        self.prompts = Some(ServerPromptsCapability::default());
        self
    }

    /// Habilita a capability `resources`.
    pub fn with_resources(mut self) -> Self {
        self.resources = Some(ServerResourcesCapability::default());
        self
    }

    /// Habilita a capability `tools`.
    pub fn with_tools(mut self) -> Self {
        self.tools = Some(ServerToolsCapability::default());
        self
    }

    /// Habilita a capability `completions`.
    pub fn with_completions(mut self) -> Self {
        self.completions = Some(Value::Object(Default::default()));
        self
    }
}

/// Capacidade do servidor de oferecer prompt templates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPromptsCapability {
    /// Servidor envia `notifications/prompts/list_changed`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub list_changed: bool,
}

/// Capacidade do servidor de fornecer resources legíveis.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerResourcesCapability {
    /// Servidor suporta subscriptions a resources individuais.
    #[serde(default, skip_serializing_if = "is_false")]
    pub subscribe: bool,
    /// Servidor envia `notifications/resources/list_changed`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub list_changed: bool,
}

/// Capacidade do servidor de expor tools invocáveis.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerToolsCapability {
    /// Servidor envia `notifications/tools/list_changed`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub list_changed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn client_capabilities_default_omits_all() {
        let capabilities = ClientCapabilities::default();

        assert_eq!(serde_json::to_value(&capabilities).unwrap(), json!({}));
    }

    #[test]
    fn client_capabilities_with_roots_serializes() {
        let capabilities = ClientCapabilities::default().with_roots();

        let value = serde_json::to_value(&capabilities).unwrap();

        assert_eq!(value, json!({ "roots": {} }));
    }

    #[test]
    fn client_capabilities_with_sampling_and_roots() {
        let capabilities = ClientCapabilities::default().with_sampling().with_roots();

        let value = serde_json::to_value(&capabilities).unwrap();

        assert_eq!(value, json!({ "sampling": {}, "roots": {} }));
    }

    #[test]
    fn client_capabilities_round_trips() {
        let capabilities = ClientCapabilities::default().with_roots();

        let json = serde_json::to_string(&capabilities).unwrap();
        let decoded: ClientCapabilities = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, capabilities);
    }

    #[test]
    fn server_capabilities_default_omits_all() {
        let capabilities = ServerCapabilities::default();

        assert_eq!(serde_json::to_value(&capabilities).unwrap(), json!({}));
    }

    #[test]
    fn server_capabilities_only_announces_enabled() {
        let capabilities = ServerCapabilities::default().with_tools();

        let value = serde_json::to_value(&capabilities).unwrap();

        assert_eq!(value, json!({ "tools": {} }));
    }

    #[test]
    fn server_capabilities_multiple() {
        let capabilities = ServerCapabilities::default()
            .with_tools()
            .with_resources()
            .with_prompts()
            .with_logging();

        let value = serde_json::to_value(&capabilities).unwrap();

        assert_eq!(
            value,
            json!({
                "tools": {},
                "resources": {},
                "prompts": {},
                "logging": {}
            })
        );
    }

    #[test]
    fn resources_capability_with_flags() {
        let capabilities = ServerResourcesCapability {
            subscribe: true,
            list_changed: true,
        };

        let value = serde_json::to_value(&capabilities).unwrap();

        assert_eq!(value, json!({ "subscribe": true, "listChanged": true }));
    }

    #[test]
    fn resources_capability_omits_false_flags() {
        let capabilities = ServerResourcesCapability::default();

        let value = serde_json::to_value(&capabilities).unwrap();

        assert_eq!(value, json!({}));
    }
}
