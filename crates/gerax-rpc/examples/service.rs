//! Serviço RPC com métodos registrados e despacho via `RpcServiceBuilder`.
use async_trait::async_trait;
use futures::executor::block_on;
use gerax_rpc::{RpcMethod, RpcRequest, RpcResponse, RpcService, RpcServiceBuilder};

struct Echo;

#[async_trait]
impl RpcMethod for Echo {
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "echo"
    }

    async fn call(&self, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        RpcResponse::success(request.payload)
    }
}

struct Upper;

#[async_trait]
impl RpcMethod for Upper {
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "upper"
    }

    async fn call(&self, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        let payload = request
            .payload
            .iter()
            .map(|b| b.to_ascii_uppercase())
            .collect();
        RpcResponse::success(payload)
    }
}

fn main() {
    let service = RpcServiceBuilder::new("demo")
        .method(Echo)
        .method(Upper)
        .build();

    let echo = block_on(service.call("echo", RpcRequest::new(b"hello".to_vec())));
    println!("echo: {:?}", echo.payload);

    let upper = block_on(service.call("upper", RpcRequest::new(b"hello".to_vec())));
    println!("upper: {:?}", upper.payload);

    let missing = block_on(service.call("nope", RpcRequest::new(Vec::new())));
    println!(
        "missing: {} - {}",
        missing.status as u8,
        missing.message.unwrap_or_default()
    );
}
