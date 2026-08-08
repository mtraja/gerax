use async_trait::async_trait;
use futures::executor::block_on;
use gerax_rpc::{
    RpcMethod, RpcRequest, RpcResponse, RpcService, RpcServiceBuilder, RpcStatus, SimpleRpcService,
};

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

struct Fail;

#[async_trait]
impl RpcMethod for Fail {
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "fail"
    }

    async fn call(&self, _request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        RpcResponse::error(RpcStatus::Internal, "boom")
    }
}

#[test]
fn service_dispatches_registered_method() {
    let mut service = SimpleRpcService::new("demo");
    service.register(Echo);

    let resp = block_on(service.call("echo", RpcRequest::new(vec![1, 2, 3])));
    assert_eq!(resp.payload, Some(vec![1, 2, 3]));
    assert_eq!(resp.status, RpcStatus::Ok);
}

#[test]
fn service_returns_not_found_for_missing_method() {
    let service = SimpleRpcService::new("demo");
    let resp = block_on(service.call("missing", RpcRequest::new(vec![])));
    assert_eq!(resp.status, RpcStatus::NotFound);
}

#[test]
fn service_builder_registers_and_dispatches() {
    let service = RpcServiceBuilder::new("built")
        .method(Echo)
        .method(Fail)
        .build();

    assert_eq!(service.name(), "built");

    let ok = block_on(service.call("echo", RpcRequest::new(vec![9])));
    assert_eq!(ok.payload, Some(vec![9]));
    assert_eq!(ok.status, RpcStatus::Ok);

    let err = block_on(service.call("fail", RpcRequest::new(vec![])));
    assert_eq!(err.status, RpcStatus::Internal);
}
