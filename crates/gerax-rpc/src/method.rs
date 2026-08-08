//! Método RPC.

use crate::request::RpcRequest;
use crate::response::RpcResponse;
use async_trait::async_trait;

/// Método RPC.
///
/// Representa uma operação invocável via RPC.
#[async_trait]
pub trait RpcMethod: Send + Sync + 'static {
    /// Tipo da requisição.
    type Request;

    /// Tipo da resposta.
    type Response;

    /// Nome do método (usado para roteamento/despacho).
    fn name(&self) -> &str;

    /// Invoca o método.
    async fn call(&self, request: RpcRequest<Self::Request>) -> RpcResponse<Self::Response>;
}
