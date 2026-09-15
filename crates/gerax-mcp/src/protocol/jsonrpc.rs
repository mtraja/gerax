//! Tipos fundamentais do JSON-RPC 2.0 usados pelo Model Context Protocol.
//!
//! Implementa o subconjunto do JSON-RPC 2.0 exigido pela especificação MCP:
//! requests, notifications, responses e errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Versão do JSON-RPC suportada.
pub const JSONRPC_VERSION: &str = "2.0";

/// Identificador de uma mensagem JSON-RPC.
///
/// A especificação MCP permite IDs numéricos ou textuais.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    /// ID numérico.
    Number(i64),
    /// ID textual.
    String(String),
    /// Ausência de ID (ex.: resposta a erro de parsing).
    Null,
}

impl JsonRpcId {
    /// Cria um ID numérico.
    pub fn number(value: i64) -> Self {
        Self::Number(value)
    }

    /// Cria um ID textual.
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }
}

impl From<i64> for JsonRpcId {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

impl std::fmt::Display for JsonRpcId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) => write!(f, "{number}"),
            Self::String(text) => f.write_str(text),
            Self::Null => f.write_str("null"),
        }
    }
}

impl From<String> for JsonRpcId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for JsonRpcId {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

/// Requisição JSON-RPC enviada por um cliente.
///
/// Representa uma chamada de método que exige resposta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Versão do protocolo JSON-RPC.
    pub jsonrpc: String,
    /// Identificador da requisição.
    pub id: JsonRpcId,
    /// Nome do método invocado.
    pub method: String,
    /// Parâmetros posicionais ou nomeados, opcionais.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Cria uma requisição JSON-RPC.
    pub fn new<I, M>(id: I, method: M, params: Option<Value>) -> Self
    where
        I: Into<JsonRpcId>,
        M: Into<String>,
    {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

/// Notification JSON-RPC.
///
/// Diferente de [`JsonRpcRequest`], não possui `id` e, por isso, não
/// deve gerar resposta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// Versão do protocolo JSON-RPC.
    pub jsonrpc: String,
    /// Nome do método notificado.
    pub method: String,
    /// Parâmetros posicionais ou nomeados, opcionais.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Cria uma notification JSON-RPC.
    pub fn new<M>(method: M, params: Option<Value>) -> Self
    where
        M: Into<String>,
    {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            method: method.into(),
            params,
        }
    }
}

/// Erro JSON-RPC.
///
/// Códigos padrão definidos na especificação JSON-RPC 2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Código numérico do erro.
    pub code: i32,
    /// Descrição legível do erro.
    pub message: String,
    /// Dados adicionais sobre o erro, opcionais.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Código JSON-RPC para erro de parsing.
    pub const PARSE_ERROR: i32 = -32700;
    /// Código JSON-RPC para requisição inválida.
    pub const INVALID_REQUEST: i32 = -32600;
    /// Código JSON-RPC para método inexistente.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Código JSON-RPC para parâmetros inválidos.
    pub const INVALID_PARAMS: i32 = -32602;
    /// Código JSON-RPC para erro interno.
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Cria um erro JSON-RPC.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Erro de parsing de mensagem.
    pub fn parse_error() -> Self {
        Self::new(Self::PARSE_ERROR, "Parse error")
    }

    /// Requisição inválida.
    pub fn invalid_request() -> Self {
        Self::new(Self::INVALID_REQUEST, "Invalid Request")
    }

    /// Método não encontrado.
    pub fn method_not_found() -> Self {
        Self::new(Self::METHOD_NOT_FOUND, "Method not found")
    }

    /// Parâmetros inválidos.
    pub fn invalid_params() -> Self {
        Self::new(Self::INVALID_PARAMS, "Invalid params")
    }

    /// Erro interno.
    pub fn internal_error() -> Self {
        Self::new(Self::INTERNAL_ERROR, "Internal error")
    }
}

/// Resposta JSON-RPC.
///
/// Uma resposta bem-sucedida contém `result`; uma resposta de erro
/// contém `error`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Versão do protocolo JSON-RPC.
    pub jsonrpc: String,
    /// Identificador da requisição correspondente.
    pub id: JsonRpcId,
    /// Resultado da operação, quando bem-sucedida.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Erro, quando a operação falhou.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Cria uma resposta de sucesso.
    pub fn success<I, V>(id: I, result: V) -> Self
    where
        I: Into<JsonRpcId>,
        V: Into<Value>,
    {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: id.into(),
            result: Some(result.into()),
            error: None,
        }
    }

    /// Cria uma resposta de erro.
    pub fn failure<I>(id: I, error: JsonRpcError) -> Self
    where
        I: Into<JsonRpcId>,
    {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }
}

/// Mensagem JSON-RPC recebida de um transporte.
///
/// O MCP envia três formas de mensagem: requests, notifications e
/// responses. Este enum desambiguiza cada forma automaticamente na
/// desserialização.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    /// Requisição que exige resposta.
    Request(JsonRpcRequest),
    /// Notification que não exige resposta.
    Notification(JsonRpcNotification),
    /// Resposta a uma requisição anterior.
    Response(JsonRpcResponse),
}

impl JsonRpcMessage {
    /// Retorna `true` se a mensagem é uma notification.
    pub fn is_notification(&self) -> bool {
        matches!(self, Self::Notification(_))
    }

    /// Retorna `true` se a mensagem é uma requisição que exige resposta.
    pub fn is_request(&self) -> bool {
        matches!(self, Self::Request(_))
    }

    /// Método da mensagem, quando aplicável (requests e notifications).
    pub fn method(&self) -> Option<&str> {
        match self {
            Self::Request(request) => Some(&request.method),
            Self::Notification(notification) => Some(&notification.method),
            Self::Response(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_with_numeric_id_serializes() {
        let request = JsonRpcRequest::new(
            JsonRpcId::number(1),
            "tools/call",
            Some(json!({ "name": "get_user" })),
        );

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["method"], "tools/call");
        assert_eq!(json["params"], json!({ "name": "get_user" }));
    }

    #[test]
    fn request_round_trips() {
        let request = JsonRpcRequest::new(
            JsonRpcId::number(7),
            "initialize",
            Some(json!({ "protocolVersion": "2025-06-18" })),
        );

        let json = serde_json::to_string(&request).unwrap();
        let decoded: JsonRpcRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, request);
    }

    #[test]
    fn request_with_string_id() {
        let request = JsonRpcRequest::new("abc-123", "ping", None);

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["id"], "abc-123");
    }

    #[test]
    fn request_without_params_omits_params() {
        let request = JsonRpcRequest::new(1, "ping", None);

        let json = serde_json::to_value(&request).unwrap();

        assert!(json.get("params").is_none());
    }

    #[test]
    fn notification_has_no_id() {
        let notification = JsonRpcNotification::new("notifications/initialized", None);

        let json = serde_json::to_value(&notification).unwrap();

        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "notifications/initialized");
        assert!(json.get("id").is_none());
        assert!(json.get("params").is_none());
    }

    #[test]
    fn notification_round_trips() {
        let notification = JsonRpcNotification::new("notifications/initialized", None);

        let json = serde_json::to_string(&notification).unwrap();
        let decoded: JsonRpcNotification = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, notification);
    }

    #[test]
    fn success_response_serializes() {
        let response = JsonRpcResponse::success(1, json!({ "name": "get_user" }));

        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["id"], 1);
        assert_eq!(json["result"], json!({ "name": "get_user" }));
        assert!(json.get("error").is_none());
    }

    #[test]
    fn success_response_with_null_result() {
        let response = JsonRpcResponse::success(1, Value::Null);

        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["result"], Value::Null);
    }

    #[test]
    fn error_response_serializes() {
        let response = JsonRpcResponse::failure(2, JsonRpcError::method_not_found());

        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["id"], 2);
        assert_eq!(json["error"]["code"], -32601);
        assert_eq!(json["error"]["message"], "Method not found");
        assert!(json.get("result").is_none());
    }

    #[test]
    fn error_with_custom_code_and_message() {
        let error = JsonRpcError::new(-32002, "Request too large");

        assert_eq!(error.code, -32002);
        assert_eq!(error.message, "Request too large");
        assert!(error.data.is_none());
    }

    #[test]
    fn invalid_json_is_a_parse_error() {
        let result: Result<JsonRpcMessage, _> = serde_json::from_str("{ not json");

        assert!(result.is_err());
    }

    #[test]
    fn message_routes_notification_without_id() {
        let json = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;

        let message: JsonRpcMessage = serde_json::from_str(json).unwrap();

        assert!(message.is_notification());

        match message {
            JsonRpcMessage::Notification(notification) => {
                assert_eq!(notification.method, "notifications/initialized");
            }
            _ => panic!("esperada uma notification"),
        }
    }

    #[test]
    fn message_routes_response() {
        let json = r#"{"jsonrpc":"2.0","id":3,"result":{"ok":true}}"#;

        let message: JsonRpcMessage = serde_json::from_str(json).unwrap();

        match message {
            JsonRpcMessage::Response(response) => {
                assert_eq!(response.id, JsonRpcId::number(3));
                assert_eq!(response.result, Some(json!({ "ok": true })));
            }
            _ => panic!("esperada uma response"),
        }
    }

    #[test]
    fn message_round_trips() {
        let request = JsonRpcRequest::new(1, "tools/list", None);
        let message = JsonRpcMessage::Request(request);

        let json = serde_json::to_string(&message).unwrap();
        let decoded: JsonRpcMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, message);
    }
}
