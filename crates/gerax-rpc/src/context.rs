//! Contexto de execução RPC.

use crate::metadata::RpcMetadata;
use std::time::Duration;

/// Contexto de execução RPC.
///
/// Carrega metadados, deadline e extensões durante o processamento
/// de uma requisição RPC.
#[derive(Debug, Clone)]
pub struct RpcContext {
    /// Metadados da requisição (headers, tracing, etc.).
    pub metadata: RpcMetadata,

    /// Tempo limite para a requisição.
    pub deadline: Option<Duration>,

    /// ID de correlação/tracing.
    pub trace_id: Option<String>,
}

impl RpcContext {
    /// Cria um novo `RpcContext` vazio.
    pub fn new() -> Self {
        Self {
            metadata: RpcMetadata::new(),
            deadline: None,
            trace_id: None,
        }
    }

    /// Define um metadado.
    pub fn with_metadata(mut self, metadata: RpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Define o tempo limite.
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Define o ID de tracing.
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

impl Default for RpcContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder de `RpcContext`.
#[derive(Debug, Clone, Default)]
pub struct RpcContextBuilder {
    metadata: RpcMetadata,
    deadline: Option<Duration>,
    trace_id: Option<String>,
}

impl RpcContextBuilder {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self {
            metadata: RpcMetadata::new(),
            deadline: None,
            trace_id: None,
        }
    }

    /// Define os metadados.
    pub fn metadata(mut self, metadata: RpcMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Define o tempo limite.
    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Define o ID de tracing.
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Constrói o `RpcContext`.
    pub fn build(self) -> RpcContext {
        RpcContext {
            metadata: self.metadata,
            deadline: self.deadline,
            trace_id: self.trace_id,
        }
    }
}
