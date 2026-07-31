use std::sync::Arc;

use gerax_http::routing::{Context, Header, HttpMethod, Request};

#[test]
fn request_parses_query_and_normalizes_header_names() {
    let mut request = Request::new(HttpMethod::Get, "/search?q=gerax".to_string(), Vec::new());
    request.headers.insert("X-Request-Id", "42");

    assert_eq!(request.path(), "/search");
    assert_eq!(request.query(), "q=gerax");
    assert_eq!(request.headers().get("x-request-id"), Some("42"));

    let ctx = Context::new(Arc::new(()), request);
    assert_eq!(
        Header::<u64>::from_name(&ctx, "X-REQUEST-ID").unwrap().0,
        42
    );
}

#[test]
fn extensions_return_owned_shared_references() {
    let ctx = Context::new(
        Arc::new(()),
        Request::new(HttpMethod::Get, "/".to_string(), Vec::new()),
    );
    ctx.extensions().insert(String::from("safe"));

    let value = ctx.extensions().get::<String>().unwrap();
    assert_eq!(value.as_str(), "safe");
    assert_eq!(
        ctx.extensions()
            .remove::<String>()
            .as_deref()
            .map(String::as_str),
        Some("safe")
    );
    assert_eq!(value.as_str(), "safe");
}
