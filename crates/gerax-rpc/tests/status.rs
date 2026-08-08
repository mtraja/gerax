use gerax_rpc::RpcStatus;

#[test]
fn code_returns_expected_values() {
    assert_eq!(RpcStatus::Ok.code(), 0);
    assert_eq!(RpcStatus::Cancelled.code(), 1);
    assert_eq!(RpcStatus::NotFound.code(), 5);
    assert_eq!(RpcStatus::Unauthenticated.code(), 16);
    assert_eq!(RpcStatus::Unknown.code(), 2);
}

#[test]
fn description_returns_expected_text() {
    assert_eq!(RpcStatus::Ok.description(), "OK");
    assert_eq!(RpcStatus::NotFound.description(), "Not Found");
    assert_eq!(
        RpcStatus::PermissionDenied.description(),
        "Permission Denied"
    );
}

#[test]
fn from_i32_roundtrips() {
    let status: RpcStatus = 5.into();
    assert_eq!(status, RpcStatus::NotFound);
    let unknown: RpcStatus = 99.into();
    assert_eq!(unknown, RpcStatus::Unknown);
    assert_eq!(RpcStatus::Unauthenticated.code(), 16);
}
