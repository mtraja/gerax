//! Servidor RPC.

use crate::service::RpcService;
use crate::transport::RpcTransport;
use async_trait::async_trait;

/// Servidor RPC genérico.
///
/// Abstrai a execução de um serviço RPC independente de protocolo.
#[async_trait]
pub trait RpcServer: Send + Sync + 'static {
    /// Inicia o servidor na porta/endereço fornecido.
    async fn serve(&self, addr: &str) -> Result<(), crate::error::RpcError>;

    /// Para o servidor.
    async fn shutdown(&self) -> Result<(), crate::error::RpcError>;
}

/// Servidor RPC baseado em transporte.
#[allow(dead_code)]
pub struct TransportRpcServer<T: RpcTransport, S: RpcService> {
    transport: T,
    service: S,
}

impl<T: RpcTransport, S: RpcService> TransportRpcServer<T, S> {
    /// Cria um novo servidor RPC com o transporte e serviço fornecidos.
    pub fn new(transport: T, service: S) -> Self {
        Self { transport, service }
    }
}

#[async_trait]
impl<T: RpcTransport + Send + Sync, S: RpcService + Send + Sync> RpcServer
    for TransportRpcServer<T, S>
{
    async fn serve(&self, _addr: &str) -> Result<(), crate::error::RpcError> {
        // Implementação simplificada
        // O servidor real aceitaria conexões e despacharia para o serviço
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), crate::error::RpcError> {
        // Implementação simplificada
        Ok(())
    }
}
