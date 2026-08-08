//! Resposta RPC.

use crate::metadata::RpcMetadata;
use crate::status::RpcStatus;

/// Resposta RPC genérica.
///
/// Independente de protocolo e serialização.
#[derive(Debug, Clone)]
pub struct RpcResponse<T> {
    /// Dados da resposta.
    pub payload: Option<T>,

    /// Status da resposta.
    pub status: RpcStatus,

    /// Mensagem de erro (se houver).
    pub message: Option<String>,

    /// Metadados (headers, etc.).
    pub metadata: RpcMetadata,
}

impl<T> RpcResponse<T> {
    /// Cria uma resposta de sucesso.
    pub fn success(payload: T) -> Self {
        Self {
            payload: Some(payload),
            status: RpcStatus::Ok,
            message: None,
            metadata: RpcMetadata::new(),
        }
    }

    /// Cria uma resposta de erro.
    pub fn error(status: RpcStatus, message: impl Into<String>) -> Self {
        Self {
            payload: None,
            status,
            message: Some(message.into()),
            metadata: RpcMetadata::new(),
        }
    }

    /// Define os metadados.
    pub fn with_metadata(mut self, metadata: RpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Verifica se a resposta é um erro.
    pub fn is_error(&self) -> bool {
        self.status != RpcStatus::Ok
    }
}

/// Builder de `RpcResponse<T>`.
#[derive(Debug, Clone, Default)]
pub struct RpcResponseBuilder<T> {
    payload: Option<T>,
    status: RpcStatus,
    message: Option<String>,
    metadata: RpcMetadata,
}

impl<T> RpcResponseBuilder<T> {
    /// Cria um novo builder com status `Ok`.
    pub fn new() -> Self {
        Self {
            payload: None,
            status: RpcStatus::Ok,
            message: None,
            metadata: RpcMetadata::new(),
        }
    }

    /// Define o payload.
    pub fn payload(mut self, payload: T) -> Self {
        self.payload = Some(payload);
        self
    }

    /// Define o status.
    pub fn status(mut self, status: RpcStatus) -> Self {
        self.status = status;
        self
    }

    /// Define a mensagem.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Define os metadados.
    pub fn metadata(mut self, metadata: RpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Constrói a resposta.
    pub fn build(self) -> RpcResponse<T> {
        RpcResponse {
            payload: self.payload,
            status: self.status,
            message: self.message,
            metadata: self.metadata,
        }
    }
}
