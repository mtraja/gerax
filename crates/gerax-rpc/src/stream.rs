//! Stream RPC.

use async_trait::async_trait;
use futures::Stream;

/// Stream RPC genérico.
///
/// Abstrai streams de dados independente de protocolo.
#[async_trait]
pub trait RpcStream<T>: Send + Sync + 'static {
    /// Recebe o próximo item do stream.
    async fn recv(&mut self) -> Option<Result<T, crate::error::RpcError>>;

    /// Envia um item no stream.
    async fn send(&mut self, item: T) -> Result<(), crate::error::RpcError>;

    /// Fecha o stream.
    async fn close(self) -> Result<(), crate::error::RpcError>;
}

/// Stream de resposta RPC (lado do servidor).
pub type RpcResponseStream<T> =
    Box<dyn Stream<Item = Result<T, crate::error::RpcError>> + Send + Unpin>;
