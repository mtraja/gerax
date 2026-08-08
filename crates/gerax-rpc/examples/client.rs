//! Cliente RPC usando um transporte em memória (`EchoTransport`).
use async_trait::async_trait;
use futures::executor::block_on;
use gerax_rpc::{RpcClient, RpcError, RpcRequest, RpcResponse, RpcTransport, TransportRpcClient};

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

fn main() {
    let client = TransportRpcClient::new(EchoTransport);
    let resp = block_on(client.call("echo", RpcRequest::new(b"ping".to_vec())));
    println!("status: {:?}", resp.status);
    println!("payload: {:?}", resp.payload);
    println!("error: {}", resp.is_error());
}
