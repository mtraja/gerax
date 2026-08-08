use gerax_rpc::{RpcExtensions, RpcRequestBuilder, RpcResponse, RpcResponseBuilder, RpcStatus};

#[test]
fn request_builder_creates_request() {
    let req = RpcRequestBuilder::new(42u32)
        .insert_metadata("k", "v")
        .build();
    assert_eq!(req.payload, 42);
    assert_eq!(req.metadata.get("k"), Some(&"v".to_string()));
}

#[test]
fn response_success_and_error() {
    let ok: RpcResponse<u8> = RpcResponse::success(1);
    assert!(!ok.is_error());
    assert_eq!(ok.status, RpcStatus::Ok);
    assert_eq!(ok.payload, Some(1));

    let err: RpcResponse<u8> = RpcResponse::error(RpcStatus::NotFound, "missing");
    assert!(err.is_error());
    assert_eq!(err.payload, None);
    assert_eq!(err.message.as_deref(), Some("missing"));
}

#[test]
fn response_builder_chains() {
    let resp: RpcResponse<u8> = RpcResponseBuilder::new()
        .status(RpcStatus::AlreadyExists)
        .message("dup")
        .build();
    assert_eq!(resp.status, RpcStatus::AlreadyExists);
    assert_eq!(resp.message.as_deref(), Some("dup"));
    assert_eq!(resp.payload, None);
}

#[test]
fn extensions_insert_get_remove() {
    let mut ext = RpcExtensions::new();
    ext.insert(10u64);
    ext.insert(String::from("hello"));
    assert_eq!(ext.get::<u64>(), Some(&10));
    assert_eq!(ext.get::<String>(), Some(&"hello".to_string()));
    assert_eq!(ext.remove::<u64>(), Some(10));
    assert_eq!(ext.get::<u64>(), None);
}
