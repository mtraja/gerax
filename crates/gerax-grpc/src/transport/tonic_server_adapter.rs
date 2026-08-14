//! Adaptador tonic para o lado servidor.
//!
//! Implementa o trait `GeraxRpc` gerado pelo `tonic-prost-build`
//! e delega as chamadas para um `dyn gerax_rpc::RpcService`.

use std::sync::Arc;

use async_trait::async_trait;
use gerax_rpc::{RpcMetadata, RpcRequest, RpcResponse};

use crate::proto::gerax_rpc_server::GeraxRpc;
use crate::proto::{RpcCallRequest, RpcCallResponse};

/// Adaptador que implementa `GeraxRpc` e delega para um `RpcService`.
pub struct GeraxRpcServerAdapter<S> {
    service: Arc<S>,
}

impl<S> GeraxRpcServerAdapter<S> {
    /// Cria um novo adaptador.
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<S> GeraxRpc for GeraxRpcServerAdapter<S>
where
    S: gerax_rpc::RpcService + Send + Sync + 'static,
{
    async fn call(
        &self,
        request: tonic::Request<RpcCallRequest>,
    ) -> Result<tonic::Response<RpcCallResponse>, tonic::Status> {
        let req = request.into_inner();

        let mut metadata = RpcMetadata::new();
        for (k, v) in req.metadata {
            metadata.insert(k, v);
        }

        let rpc_request = RpcRequest::new(req.payload).with_metadata(metadata);

        let response: RpcResponse<Vec<u8>> = self.service.call(&req.method, rpc_request).await;

        let status = response.status;
        let payload = response.payload.unwrap_or_default();
        let message = response
            .message
            .unwrap_or_else(|| status.description().to_string());

        Ok(tonic::Response::new(RpcCallResponse {
            payload,
            status: status.code(),
            message,
        }))
    }
}
