//! Serviço RPC.

use crate::method::RpcMethod;
use crate::request::RpcRequest;
use crate::response::RpcResponse;
use crate::status::RpcStatus;
use async_trait::async_trait;
use std::collections::HashMap;

type BoxedMethod = Box<dyn RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + Send + Sync>;

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
        M: RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + 'static;

    /// Invoca um método pelo nome.
    async fn call(&self, method: &str, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>>;
}

/// Serviço RPC genérico simples.
pub struct SimpleRpcService {
    name: String,
    methods: HashMap<String, BoxedMethod>,
}

impl SimpleRpcService {
    /// Cria um novo serviço simples.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            methods: HashMap::new(),
        }
    }
}

#[async_trait]
impl RpcService for SimpleRpcService {
    fn name(&self) -> &str {
        &self.name
    }

    fn register<M>(&mut self, method: M)
    where
        M: RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + 'static,
    {
        self.methods
            .insert(method.name().to_string(), Box::new(method));
    }

    async fn call(&self, method: &str, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        match self.methods.get(method) {
            Some(handler) => handler.call(request).await,
            None => {
                RpcResponse::error(RpcStatus::NotFound, format!("method not found: {}", method))
            }
        }
    }
}

/// Builder de serviço RPC.
pub struct RpcServiceBuilder {
    name: String,
    methods: Vec<(String, BoxedMethod)>,
}

impl RpcServiceBuilder {
    /// Cria um novo builder para um serviço.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            methods: Vec::new(),
        }
    }

    /// Registra um método no serviço.
    pub fn method<M>(mut self, method: M) -> Self
    where
        M: RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + 'static,
    {
        self.methods
            .push((method.name().to_string(), Box::new(method)));
        self
    }

    /// Constrói o serviço.
    pub fn build(self) -> SimpleRpcService {
        let mut service = SimpleRpcService::new(self.name);
        for (name, method) in self.methods {
            service.methods.insert(name, method);
        }
        service
    }
}
