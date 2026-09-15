//! Notifications do Model Context Protocol.
//!
//! Notifications são mensagens JSON-RPC sem `id` e, portanto, não
//! geram resposta.

use serde_json::json;

use super::jsonrpc::JsonRpcNotification;

/// Método de notification `notifications/initialized`.
pub const NOTIFICATIONS_INITIALIZED: &str = "notifications/initialized";

/// Método de notification `notifications/tools/list_changed`.
pub const NOTIFICATIONS_TOOLS_LIST_CHANGED: &str = "notifications/tools/list_changed";

/// Método de notification `notifications/resources/list_changed`.
pub const NOTIFICATIONS_RESOURCES_LIST_CHANGED: &str = "notifications/resources/list_changed";

/// Método de notification `notifications/prompts/list_changed`.
pub const NOTIFICATIONS_PROMPTS_LIST_CHANGED: &str = "notifications/prompts/list_changed";

/// Método de notification `notifications/message` (logging).
pub const NOTIFICATIONS_MESSAGE: &str = "notifications/message";

/// Níveis de log do `notifications/message`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// `"debug"`
    Debug,
    /// `"info"`
    Info,
    /// `"notice"`
    Notice,
    /// `"warning"`
    Warning,
    /// `"error"`
    Error,
    /// `"critical"`
    Critical,
    /// `"alert"`
    Alert,
    /// `"emergency"`
    Emergency,
}

impl LogLevel {
    /// Serializa o nível para o valor usado pelo protocolo.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Notice => "notice",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
            Self::Alert => "alert",
            Self::Emergency => "emergency",
        }
    }
}

/// Cria a notification `notifications/initialized`.
///
/// É enviada pelo cliente ao servidor após receber a resposta de
/// `initialize`, indicando que está pronto para a fase de operação.
pub fn initialized_notification() -> JsonRpcNotification {
    JsonRpcNotification::new(NOTIFICATIONS_INITIALIZED, None)
}

/// Cria a notification `notifications/tools/list_changed`.
pub fn tools_list_changed_notification() -> JsonRpcNotification {
    JsonRpcNotification::new(NOTIFICATIONS_TOOLS_LIST_CHANGED, None)
}

/// Cria a notification `notifications/resources/list_changed`.
pub fn resources_list_changed_notification() -> JsonRpcNotification {
    JsonRpcNotification::new(NOTIFICATIONS_RESOURCES_LIST_CHANGED, None)
}

/// Cria a notification `notifications/prompts/list_changed`.
pub fn prompts_list_changed_notification() -> JsonRpcNotification {
    JsonRpcNotification::new(NOTIFICATIONS_PROMPTS_LIST_CHANGED, None)
}

/// Cria a notification `notifications/message` (logging estruturado).
pub fn message_notification(level: LogLevel, message: impl Into<String>) -> JsonRpcNotification {
    JsonRpcNotification::new(
        NOTIFICATIONS_MESSAGE,
        Some(json!({
            "level": level.as_str(),
            "message": message.into()
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::jsonrpc::JsonRpcMessage;

    #[test]
    fn initialized_notification_has_no_id() {
        let notification = initialized_notification();

        let value = serde_json::to_value(&notification).unwrap();

        assert_eq!(value["method"], "notifications/initialized");
        assert!(value.get("id").is_none());
        assert!(value.get("params").is_none());
    }

    #[test]
    fn initialized_notification_routes_as_notification() {
        let json = serde_json::to_string(&initialized_notification()).unwrap();

        let message: JsonRpcMessage = serde_json::from_str(&json).unwrap();

        assert!(message.is_notification());
    }
}
