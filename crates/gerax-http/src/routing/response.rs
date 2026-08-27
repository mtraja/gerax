use super::HeaderMap;
use crate::HttpServerError;
use serde::Serialize;

#[derive(Default)]
pub struct Response {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Response {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: HeaderMap::new(),
            body: body.into(),
        }
    }

    pub fn json<T: Serialize>(body: T) -> Result<Self, HttpServerError> {
        let body = serde_json::to_vec(&body)
            .map_err(|e| HttpServerError::HandlerError(e.to_string()))?;
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json");
        Ok(Self {
            status: 200,
            headers,
            body,
        })
    }

    pub fn not_found() -> Self {
        Self {
            status: 404,
            headers: HeaderMap::new(),
            body: b"Not Found".to_vec(),
        }
    }
}
