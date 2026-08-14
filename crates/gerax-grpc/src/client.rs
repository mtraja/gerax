use std::marker::PhantomData;
use std::sync::Arc;

use gerax_codec::Codec;
use gerax_core::Entity;
use serde::{Deserialize, Serialize};

use crate::error::GrpcError;
use crate::transport::GrpcInvoker;

/// Invocador gRPC real usando `tonic`.
struct TonicRpcInvoker {
    channel: tonic::transport::Channel,
}

impl TonicRpcInvoker {
    fn new(channel: tonic::transport::Channel) -> Self {
        Self { channel }
    }
}

#[async_trait::async_trait]
impl GrpcInvoker for TonicRpcInvoker {
    async fn invoke(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError> {
        let mut client = crate::proto::gerax_rpc_client::GeraxRpcClient::new(self.channel.clone());

        let response = client
            .call(tonic::Request::new(crate::proto::RpcCallRequest {
                method: method.to_string(),
                payload,
                metadata: std::collections::HashMap::new(),
            }))
            .await
            .map_err(|e| GrpcError::RpcError(e.to_string()))?;

        let inner = response.into_inner();
        if inner.status != 0 {
            return Err(GrpcError::RpcError(if inner.message.is_empty() {
                "unknown error".to_string()
            } else {
                inner.message
            }));
        }

        Ok(inner.payload)
    }
}

/// Cliente gRPC genérico.
///
/// Responsável por consumir serviços gRPC e converter mensagens
/// para/from entidades `gerax-core`.
pub struct GrpcClient<T, C = gerax_codec::JsonCodec> {
    endpoint: String,
    #[allow(dead_code)]
    codec: Arc<C>,
    channel: std::sync::Mutex<Option<tonic::transport::Channel>>,
    _marker: PhantomData<T>,
}

impl<T, C> GrpcClient<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Send + Sync + 'static,
{
    /// Cria um novo cliente gRPC.
    pub fn new(endpoint: impl Into<String>, codec: C) -> Self {
        Self {
            endpoint: endpoint.into(),
            codec: Arc::new(codec),
            channel: std::sync::Mutex::new(None),
            _marker: PhantomData,
        }
    }

    /// Conecta ao servidor gRPC no endereço fornecido.
    pub async fn connect(&self) -> Result<(), GrpcError> {
        let endpoint = tonic::transport::Endpoint::from_shared(self.endpoint.clone())
            .map_err(|e| GrpcError::RpcError(format!("invalid endpoint: {}", e)))?;

        let conn = endpoint
            .connect()
            .await
            .map_err(|e| GrpcError::RpcError(format!("tonic connect error: {}", e)))?;

        let mut guard = self.channel.lock().unwrap();
        *guard = Some(conn);
        Ok(())
    }

    /// Invoca um método gRPC remotamente com payload tipado.
    ///
    /// O mesmo tipo é usado para a requisição e para a resposta.
    pub async fn call<R>(&self, method: &str, request: R) -> Result<R, GrpcError>
    where
        R: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    {
        let bytes = serde_json::to_vec(&request)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))?;

        let response_bytes = self.send_request(method, bytes).await?;

        serde_json::from_slice(&response_bytes)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))
    }

    /// Atalho para `read_<collection>` (busca por id).
    pub async fn find_by_id(&self, id: &str) -> Result<Option<T>, GrpcError> {
        let method = format!("read_{}", T::collection_name());
        match self
            .send_request(&method, id.to_string().into_bytes())
            .await
        {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| GrpcError::SerializationError(e.to_string())),
            Err(GrpcError::RpcError(msg)) if msg.starts_with("entity not found") => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Atalho para `list_<collection>` (lista todas as entidades).
    pub async fn find_all(&self) -> Result<Vec<T>, GrpcError> {
        let method = format!("list_{}", T::collection_name());
        let bytes =
            serde_json::to_vec(&()).map_err(|e| GrpcError::SerializationError(e.to_string()))?;

        let response_bytes = self.send_request(&method, bytes).await?;

        serde_json::from_slice(&response_bytes)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))
    }

    /// Atalho para `create_<collection>` (insere entidade).
    pub async fn insert(&self, entity: T) -> Result<T, GrpcError> {
        let method = format!("create_{}", T::collection_name());
        self.call(&method, entity).await
    }

    /// Atalho para `update_<collection>` (atualiza entidade).
    pub async fn update(&self, entity: T) -> Result<(), GrpcError> {
        let method = format!("update_{}", T::collection_name());
        let bytes = serde_json::to_vec(&entity)
            .map_err(|e| GrpcError::SerializationError(e.to_string()))?;

        self.send_request(&method, bytes).await?;
        Ok(())
    }

    /// Atalho para `delete_<collection>` (remove por id).
    pub async fn delete(&self, id: &str) -> Result<(), GrpcError> {
        let method = format!("delete_{}", T::collection_name());
        self.send_request(&method, id.to_string().into_bytes())
            .await?;
        Ok(())
    }

    /// Envia uma requisição raw em bytes e retorna a resposta raw em bytes.
    async fn send_request(&self, method: &str, payload: Vec<u8>) -> Result<Vec<u8>, GrpcError> {
        let channel = {
            let guard = self.channel.lock().unwrap();
            guard.clone().ok_or_else(|| {
                GrpcError::RpcError("client not connected, call connect() first".into())
            })?
        };

        let invoker = TonicRpcInvoker::new(channel);
        invoker.invoke(method, payload).await
    }
}
