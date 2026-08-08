//! Cliente RPC.

use crate::request::RpcRequest;
use crate::response::RpcResponse;
use crate::transport::RpcTransport;
use async_trait::async_trait;

/// Cliente RPC genérico.
///
/// Abstrai a chamada RPC independente de protocolo.
#[async_trait]
pub trait RpcClient: Send + Sync + 'static {
    /// Tipo da requisição serializada.
    type Request;

    /// Tipo da resposta deserializada.
    type Response;

    /// Invoca um método remotamente.
    async fn call(
        &self,
        method: &str,
        request: RpcRequest<Self::Request>,
    ) -> RpcResponse<Self::Response>;
}

/// Cliente RPC baseado em transporte.
pub struct TransportRpcClient<T: RpcTransport> {
    transport: T,
}

impl<T: RpcTransport> TransportRpcClient<T> {
    /// Cria um novo cliente RPC com o transporte fornecido.
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl<T: RpcTransport + Clone + Send + Sync> RpcClient for TransportRpcClient<T>
where
    T::Connection: Send + Sync,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    async fn call(
        &self,
        _method: &str,
        request: RpcRequest<Self::Request>,
    ) -> RpcResponse<Self::Response> {
        match self.transport.connect().await {
            Ok(mut conn) => match self.transport.send(&mut conn, request).await {
                Ok(response) => response,
                Err(e) => RpcResponse::error(
                    crate::status::RpcStatus::Internal,
                    format!("Transport error: {}", e),
                ),
            },
            Err(e) => RpcResponse::error(
                crate::status::RpcStatus::Internal,
                format!("Connection error: {}", e),
            ),
        }
    }
}
