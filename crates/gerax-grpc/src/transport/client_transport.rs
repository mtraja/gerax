//! Transporte gRPC cliente usando `tonic`.

use async_trait::async_trait;
use gerax_codec::Codec;
use gerax_rpc::{RpcTransport, error::RpcError, request::RpcRequest, response::RpcResponse};

use crate::error::GrpcError;

/// Stub genérico para invocar métodos gRPC.
///
/// Permite que `TonicClientTransport` delega a chamada efetiva do método
/// para uma implementação concreta (gerada por `tonic-build` ou construída
/// dinamicamente), mantendo o transporte independente do serviço.
#[async_trait]
pub trait GrpcInvoker: Send + Sync + 'static {
    /// Invoca um método gRPC pelo nome e retorna o payload da resposta.
    async fn invoke(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError>;
}

/// Stub de invocação com suporte a codec.
///
/// Combina um `GrpcInvoker` com um `Codec` para serializar a requisição
/// e deserializar a resposta automaticamente.
#[async_trait]
pub trait GrpcCodecInvoker<C, T>: Send + Sync + 'static
where
    C: Codec<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    /// Invoca um método gRPC com payload tipado.
    async fn invoke_typed(&self, method: &str, payload: T, codec: &C) -> Result<T, GrpcError>;
}

#[async_trait]
impl<I, C, T> GrpcCodecInvoker<C, T> for I
where
    I: GrpcInvoker + Send + Sync,
    C: Codec<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    async fn invoke_typed(&self, method: &str, payload: T, codec: &C) -> Result<T, GrpcError> {
        let bytes = codec
            .serialize(&payload)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))?;
        let response_bytes = self.invoke(method, bytes).await?;
        codec
            .deserialize(&response_bytes)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))
    }
}

/// Transporte gRPC cliente baseado em `tonic`.
///
/// Implementa `gerax_rpc::RpcTransport` para o lado cliente.
///
/// # Type parameters
///
/// - `I`: implementação de `GrpcInvoker` que efetivamente chama o método gRPC.
pub struct TonicClientTransport<I> {
    /// Endpoint gRPC (ex: `http://[::1]:50051`).
    pub endpoint: String,
    /// Stub de invocação de métodos.
    pub invoker: I,
}

impl<I> TonicClientTransport<I> {
    /// Cria um novo transporte cliente.
    pub fn new(endpoint: impl Into<String>, invoker: I) -> Self {
        Self {
            endpoint: endpoint.into(),
            invoker,
        }
    }
}

#[async_trait]
impl<I: GrpcInvoker + Send + Sync> RpcTransport for TonicClientTransport<I> {
    type Connection = tonic::transport::Channel;

    async fn connect(&self) -> Result<Self::Connection, RpcError> {
        let endpoint = tonic::transport::Endpoint::from_shared(self.endpoint.clone())
            .map_err(|e| RpcError::Transport(format!("invalid endpoint: {}", e)))?;

        endpoint
            .connect()
            .await
            .map_err(|e| RpcError::Transport(format!("tonic connect error: {}", e)))
    }

    async fn send(
        &self,
        _conn: &mut Self::Connection,
        request: RpcRequest<Vec<u8>>,
    ) -> Result<RpcResponse<Vec<u8>>, RpcError> {
        let method = request
            .metadata
            .get("grpc-method")
            .map(|s| s.as_str())
            .unwrap_or("");

        let payload = self
            .invoker
            .invoke(method, request.payload)
            .await
            .map_err(|e| RpcError::Internal(format!("grpc invoke error: {}", e)))?;

        Ok(RpcResponse::success(payload))
    }

    async fn close(&self, conn: Self::Connection) -> Result<(), RpcError> {
        drop(conn);
        Ok(())
    }
}
