use async_trait::async_trait;
use futures::executor::block_on;
use gerax_rpc::{
    RpcClient, RpcError, RpcRequest, RpcResponse, RpcStatus, RpcTransport, TransportRpcClient,
};

#[derive(Clone)]
struct EchoTransport;

#[async_trait]
impl RpcTransport for EchoTransport {
    type Connection = ();

    async fn connect(&self) -> Result<(), RpcError> {
        Ok(())
    }

    async fn send(
        &self,
        _conn: &mut (),
        request: RpcRequest<Vec<u8>>,
    ) -> Result<RpcResponse<Vec<u8>>, RpcError> {
        Ok(RpcResponse::success(request.payload))
    }

    async fn close(&self, _conn: ()) -> Result<(), RpcError> {
        Ok(())
    }
}

#[test]
fn transport_client_roundtrips_payload() {
    let client = TransportRpcClient::new(EchoTransport);
    let resp = block_on(client.call("echo", RpcRequest::new(vec![7, 8])));
    assert_eq!(resp.payload, Some(vec![7, 8]));
    assert_eq!(resp.status, RpcStatus::Ok);
}
