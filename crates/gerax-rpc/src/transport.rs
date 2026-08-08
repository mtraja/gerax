//! Transporte RPC.

use crate::request::RpcRequest;
use crate::response::RpcResponse;
use async_trait::async_trait;

/// Transporte RPC.
///
/// Abstrai a camada de transporte (TCP, QUIC, HTTP/2, etc.)
/// independente de protocolo.
#[async_trait]
pub trait RpcTransport: Send + Sync + 'static {
    /// Tipo da conexão.
    type Connection: Send + Sync + 'static;

    /// Estabelece uma conexão.
    async fn connect(&self) -> Result<Self::Connection, crate::error::RpcError>;

    /// Envia uma requisição e recebe uma resposta.
    async fn send(
        &self,
        conn: &mut Self::Connection,
        request: RpcRequest<Vec<u8>>,
    ) -> Result<RpcResponse<Vec<u8>>, crate::error::RpcError>;

    /// Fecha a conexão.
    async fn close(&self, conn: Self::Connection) -> Result<(), crate::error::RpcError>;
}
