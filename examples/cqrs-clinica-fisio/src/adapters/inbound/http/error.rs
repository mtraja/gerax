use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use gerax_cqrs::CqrsError;
use serde_json::json;

/// Erro de API: status HTTP + mensagem.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

/// Converte os erros do `gerax-cqrs` em respostas HTTP.
///
/// Erros produzidos pelos handlers carregam uma tag de categoria na
/// mensagem (`[not_found]`, `[business_rule]`, `[infrastructure]`),
/// definida por [`crate::application::ApplicationError::into_cqrs`],
/// que é traduzida para o status code.
impl From<CqrsError> for ApiError {
    fn from(err: CqrsError) -> Self {
        match err {
            CqrsError::Execution(msg) => {
                let (status, message) = if let Some(rest) = msg.strip_prefix("[not_found] ") {
                    (StatusCode::NOT_FOUND, rest.to_string())
                } else if let Some(rest) = msg.strip_prefix("[business_rule] ") {
                    (StatusCode::BAD_REQUEST, rest.to_string())
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, msg)
                };
                Self::new(status, message)
            }
            _ => Self::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}