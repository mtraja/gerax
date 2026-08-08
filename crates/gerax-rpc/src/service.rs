//! Serviço RPC.

use crate::method::RpcMethod;
use crate::request::RpcRequest;
use crate::response::RpcResponse;
use async_trait::async_trait;

/// Serviço RPC.
///
/// Agrupa métodos e faz o despacho de requisições.
#[async_trait]
pub trait RpcService: Send + Sync + 'static {
    /// Nome do serviço.
    fn name(&self) -> &str;

    /// Registra um método no serviço.
    fn register<M>(&mut self, method: M)
    where
        M: RpcMethod + 'static;

    /// Invoca um método pelo nome.
    async fn call(&self, method: &str, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>>;
}

/// Serviço RPC genérico simples.
pub struct SimpleRpcService {
    name: String,
}

impl SimpleRpcService {
    /// Cria um novo serviço simples.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl RpcService for SimpleRpcService {
    fn name(&self) -> &str {
        &self.name
    }

    fn register<M>(&mut self, _method: M)
    where
        M: RpcMethod + 'static,
    {
        // Implementação simplificada
        // O registro real seria feito aqui
    }

    async fn call(&self, _method: &str, _request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        RpcResponse::error(
            crate::status::RpcStatus::Unimplemented,
            "SimpleRpcService::call not implemented",
        )
    }
}
