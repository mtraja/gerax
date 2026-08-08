//! Requisição RPC.

use crate::context::RpcContext;
use crate::metadata::RpcMetadata;

/// Requisição RPC genérica.
///
/// Independente de protocolo e serialização.
#[derive(Debug, Clone)]
pub struct RpcRequest<T> {
    /// Dados da requisição.
    pub payload: T,

    /// Metadados (headers, etc.).
    pub metadata: RpcMetadata,

    /// Contexto de execução.
    pub context: RpcContext,
}

impl<T> RpcRequest<T> {
    /// Cria uma nova requisição com payload.
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            metadata: RpcMetadata::new(),
            context: RpcContext::new(),
        }
    }

    /// Define os metadados.
    pub fn with_metadata(mut self, metadata: RpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Define o contexto.
    pub fn with_context(mut self, context: RpcContext) -> Self {
        self.context = context;
        self
    }
}

/// Builder de `RpcRequest<T>`.
#[derive(Debug, Clone)]
pub struct RpcRequestBuilder<T> {
    payload: T,
    metadata: RpcMetadata,
    context: RpcContext,
}

impl<T> RpcRequestBuilder<T> {
    /// Cria um novo builder com payload.
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            metadata: RpcMetadata::new(),
            context: RpcContext::new(),
        }
    }

    /// Define os metadados.
    pub fn metadata(mut self, metadata: RpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Insere um metadado.
    pub fn insert_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Define o contexto.
    pub fn context(mut self, context: RpcContext) -> Self {
        self.context = context;
        self
    }

    /// Constrói a requisição.
    pub fn build(self) -> RpcRequest<T> {
        RpcRequest {
            payload: self.payload,
            metadata: self.metadata,
            context: self.context,
        }
    }
}
