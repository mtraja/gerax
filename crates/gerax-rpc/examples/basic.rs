//! Uso básico de tipos síncronos do gerax-rpc.
use gerax_rpc::{
    RpcContextBuilder, RpcExtensions, RpcMetadataBuilder, RpcRequestBuilder, RpcResponse,
    RpcResponseBuilder, RpcStatus,
};

fn main() {
    let metadata = RpcMetadataBuilder::new()
        .insert("authorization", "Bearer token")
        .insert("trace-id", "abc123")
        .build();

    let context = RpcContextBuilder::new()
        .metadata(metadata.clone())
        .trace_id("abc123")
        .build();

    let request = RpcRequestBuilder::new("ping".to_string())
        .metadata(metadata)
        .context(context)
        .build();

    println!("payload: {}", request.payload);
    println!("trace-id: {:?}", request.context.trace_id);

    let ok: RpcResponse<String> = RpcResponse::success("pong".to_string());
    assert!(!ok.is_error());
    assert_eq!(ok.status, RpcStatus::Ok);

    let error = RpcResponseBuilder::<String>::new()
        .status(RpcStatus::NotFound)
        .message("recurso não encontrado")
        .build();
    assert!(error.is_error());
    assert_eq!(error.status, RpcStatus::NotFound);

    let mut extensions = RpcExtensions::new();
    extensions.insert(42u64);
    println!("extensions: {:?}", extensions.get::<u64>());
}
