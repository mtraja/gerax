use gerax_rpc::{RpcContext, RpcContextBuilder, RpcMetadata, RpcMetadataBuilder};
use std::time::Duration;

#[test]
fn metadata_insert_get_remove() {
    let mut meta = RpcMetadata::new();
    meta.insert("a", "1");
    meta.insert("b", "2");
    assert_eq!(meta.get("a"), Some(&"1".to_string()));
    assert!(meta.contains_key("b"));
    assert_eq!(meta.remove("b"), Some("2".to_string()));
    assert!(!meta.contains_key("b"));
}

#[test]
fn metadata_iter_count() {
    let mut meta = RpcMetadata::new();
    meta.insert("x", "1");
    meta.insert("y", "2");
    assert_eq!(meta.iter().count(), 2);
}

#[test]
fn metadata_from_hashmap() {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert("k".to_string(), "v".to_string());
    let meta: RpcMetadata = map.into();
    assert_eq!(meta.get("k"), Some(&"v".to_string()));
}

#[test]
fn metadata_builder_chains() {
    let meta = RpcMetadataBuilder::new()
        .insert("k1", "v1")
        .insert("k2", "v2")
        .build();
    assert_eq!(meta.get("k1"), Some(&"v1".to_string()));
    assert_eq!(meta.get("k2"), Some(&"v2".to_string()));
}

#[test]
fn context_fields_and_with() {
    let ctx = RpcContext::new()
        .with_deadline(Duration::from_secs(30))
        .with_trace_id("t1");
    assert_eq!(ctx.deadline, Some(Duration::from_secs(30)));
    assert_eq!(ctx.trace_id.as_deref(), Some("t1"));
}

#[test]
fn context_builder_chains() {
    let ctx = RpcContextBuilder::new()
        .deadline(Duration::from_secs(5))
        .trace_id("abc")
        .build();
    assert_eq!(ctx.deadline, Some(Duration::from_secs(5)));
    assert_eq!(ctx.trace_id.as_deref(), Some("abc"));
}
