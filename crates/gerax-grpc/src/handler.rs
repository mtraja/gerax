//! Handler HTTP para expor serviços gRPC via `gerax-http`.
//!
//! O `GrpcHandler` implementa `gerax_http::routing::Handler` e atua como
//! um proxy RPC: cada requisição HTTP é despachada para o `GrpcService`
//! subjacente usando o nome do método informado no header `grpc-method`.
//! O corpo da requisição é o payload serializado da chamada e o corpo da
//! resposta HTTP é o payload serializado da resposta RPC.

use std::sync::Arc;

use async_trait::async_trait;
use gerax_codec::Codec;
use gerax_core::Entity;
use gerax_http::routing::{Context, Handler, Response};
use gerax_http::{HttpServerError, ServerResult};
use gerax_rpc::{RpcRequest, RpcService, status::RpcStatus};

use crate::method::GrpcService;

/// Método HTTP usado para invocar o proxy gRPC.
const GRPC_METHOD_HEADER: &str = "grpc-method";

/// Handler HTTP que expõe um `GrpcService<T, C>` como proxy RPC.
///
/// Registre-o em um `Router` (ver [`GrpcHandler::register`]) e envie
/// requisições cujo header `grpc-method` indique o método a invocar.
pub struct GrpcHandler<T, C> {
    service: Arc<GrpcService<T, C>>,
}

impl<T, C> GrpcHandler<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Codec<Vec<T>> + Send + Sync + 'static,
{
    /// Cria um novo handler a partir de um `GrpcService`.
    pub fn new(service: GrpcService<T, C>) -> Self {
        Self {
            service: Arc::new(service),
        }
    }

    /// Cria um novo handler a partir de um `GrpcService` compartilhado.
    pub fn from_service(service: Arc<GrpcService<T, C>>) -> Self {
        Self { service }
    }

    /// Registra o handler em um `Router` no path informado (método HTTP POST).
    pub fn register<State>(
        self,
        router: gerax_http::Router<State>,
        path: impl Into<String>,
    ) -> gerax_http::Router<State>
    where
        State: Send + Sync + 'static,
    {
        router.post(path, self)
    }

    fn map_status(status: RpcStatus) -> u16 {
        match status {
            RpcStatus::Ok => 200,
            RpcStatus::InvalidArgument => 400,
            RpcStatus::NotFound => 404,
            RpcStatus::AlreadyExists => 409,
            RpcStatus::Unauthenticated => 401,
            RpcStatus::PermissionDenied => 403,
            RpcStatus::Unimplemented => 501,
            RpcStatus::Unavailable => 503,
            _ => 500,
        }
    }
}

#[async_trait]
impl<T, C, State> Handler<State> for GrpcHandler<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Codec<Vec<T>> + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    async fn call(&self, ctx: Context<State>) -> ServerResult<Response> {
        let method = ctx
            .request()
            .headers()
            .get(GRPC_METHOD_HEADER)
            .ok_or_else(|| {
                HttpServerError::HandlerError(format!("missing '{}' header", GRPC_METHOD_HEADER))
            })?;

        let request = RpcRequest::new(ctx.request().body().to_vec());
        let response = self.service.call(method, request).await;

        if response.status == RpcStatus::Ok {
            Ok(Response::ok(response.payload.unwrap_or_default()))
        } else {
            let message = response
                .message
                .unwrap_or_else(|| response.status.description().to_string());
            Ok(Response {
                status: Self::map_status(response.status),
                body: message.into_bytes(),
                ..Default::default()
            })
        }
    }
}
