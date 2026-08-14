//! Transporte gRPC servidor usando `tonic`.

use async_trait::async_trait;
use gerax_codec::Codec;
use gerax_rpc::{RpcTransport, error::RpcError, request::RpcRequest, response::RpcResponse};

use crate::error::GrpcError;

/// Handler para requisições gRPC no lado servidor.
///
/// Permite que `TonicServerTransport` delega o processamento da requisição
/// para uma implementação concreta (ex: adaptador `RpcService` na Fase 4).
#[async_trait]
pub trait GrpcRequestHandler: Send + Sync + 'static {
    /// Processa uma requisição gRPC e retorna o payload da resposta.
    async fn handle(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError>;
}

/// Handler de requisição com suporte a codec.
///
/// Combina um `GrpcRequestHandler` com um `Codec` para deserializar a requisição
/// e serializar a resposta automaticamente.
#[async_trait]
pub trait GrpcCodecHandler<C, T>: Send + Sync + 'static
where
    C: Codec<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    /// Processa uma requisição gRPC com payload tipado.
    async fn handle_typed(&self, method: &str, payload: T, codec: &C) -> Result<T, GrpcError>;
}

#[async_trait]
impl<H, C, T> GrpcCodecHandler<C, T> for H
where
    H: GrpcRequestHandler + Send + Sync,
    C: Codec<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    async fn handle_typed(&self, method: &str, payload: T, codec: &C) -> Result<T, GrpcError> {
        let bytes = codec
            .serialize(&payload)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))?;
        let response_bytes = self.handle(method, bytes).await?;
        codec
            .deserialize(&response_bytes)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))
    }
}

/// Handle de conexão do transporte servidor.
///
/// Encapsula o estado de uma conexão/stream no lado servidor.
/// Por enquanto é um wrapper simples; nas fases seguintes pode ser
/// estendido para manter metadados da conexão tonic.
#[derive(Debug, Clone, Default)]
pub struct TonicServerConnection {
    _marker: std::marker::PhantomData<()>,
}

impl TonicServerConnection {
    /// Cria uma nova conexão servidor vazia.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

/// Transporte gRPC servidor baseado em `tonic`.
///
/// Implementa `gerax_rpc::RpcTransport` para o lado servidor.
///
/// # Type parameters
///
/// - `H`: implementação de `GrpcRequestHandler` que processa as requisições.
pub struct TonicServerTransport<H> {
    /// Handler de requisições.
    pub handler: H,
}

impl<H> TonicServerTransport<H> {
    /// Cria um novo transporte servidor.
    pub fn new(handler: H) -> Self {
        Self { handler }
    }
}

#[async_trait]
impl<H: GrpcRequestHandler + Send + Sync> RpcTransport for TonicServerTransport<H> {
    type Connection = TonicServerConnection;

    async fn connect(&self) -> Result<Self::Connection, RpcError> {
        // No modelo atual, o servidor tonic aceita conexões incoming
        // de forma automática. Aqui retornamos uma conexão placeholder
        // que será usada pelo `send` para despachar a requisição.
        Ok(TonicServerConnection::new())
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
            .handler
            .handle(method, request.payload)
            .await
            .map_err(|e| RpcError::Internal(format!("grpc handler error: {}", e)))?;

        Ok(RpcResponse::success(payload))
    }

    async fn close(&self, _conn: Self::Connection) -> Result<(), RpcError> {
        // Placeholder: nas fases seguintes pode finalizar stream/contexto.
        Ok(())
    }
}
