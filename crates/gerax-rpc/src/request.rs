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
