//! Testes de integração do transporte HTTP MCP.

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use gerax_mcp::{CallToolResult, McpServer, tool};
use gerax_mcp_http::{McpHttpServer, OriginPolicy};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

fn server_with_tool() -> McpServer {
    McpServer::builder()
        .name("http-test")
        .version("1.0")
        .tool(tool(
            "echo",
            "Repete o texto",
            json!({ "type": "object", "properties": { "text": { "type": "string" } } }),
            |args| async move {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned();
                Ok(CallToolResult::text(text))
            },
        ))
        .build()
}

fn build_app(server: McpServer) -> (Router, McpHttpServer) {
    let http = McpHttpServer::for_server(server).build();
    (http.router(), http)
}

async fn send(
    app: &Router,
    method: Method,
    body: Option<Value>,
    session: Option<&str>,
    accept: &str,
    origin: Option<&str>,
    protocol_version: Option<&str>,
) -> (StatusCode, String, HeaderMap) {
    let mut builder = Request::builder()
        .method(method)
        .uri("/mcp")
        .header("accept", accept)
        .header("content-type", "application/json");
    if let Some(sid) = session {
        builder = builder.header("mcp-session-id", sid);
    }
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    if let Some(version) = protocol_version {
        builder = builder.header("mcp-protocol-version", version);
    }
    let request = match body {
        Some(body) => builder.body(Body::from(body.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        String::from_utf8_lossy(&bytes).into_owned(),
        headers,
    )
}

fn initialize_body() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0" }
        }
    })
}

async fn initialize(app: &Router, http: &McpHttpServer) -> String {
    let (status, body, _) = send(
        app,
        Method::POST,
        Some(initialize_body()),
        None,
        "application/json",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["result"]["serverInfo"]["name"], "http-test");
    http.session_id().expect("sessão criada após initialize")
}

async fn initialized_notification(app: &Router, session: &str) {
    let (status, body, _) = send(
        app,
        Method::POST,
        Some(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })),
        Some(session),
        "application/json",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn initialize_returns_session_and_server_info() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;

    assert!(!session.is_empty());
    assert_eq!(http.session_id().as_deref(), Some(session.as_str()));
}

#[tokio::test]
async fn request_without_session_is_400() {
    let (app, _http) = build_app(server_with_tool());

    let (status, body, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
        None,
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["error"]["code"], -32600);
}

#[tokio::test]
async fn request_with_unknown_session_is_404() {
    let (app, _http) = build_app(server_with_tool());

    let (status, body, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
        Some("sessao-desconhecida"),
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["error"]["code"], -32600);
}

#[tokio::test]
async fn tools_list_before_initialized_is_server_not_initialized() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;

    let (status, body, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
        Some(&session),
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["error"]["code"], -32000);
}

#[tokio::test]
async fn initialized_notification_is_accepted() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;

    initialized_notification(&app, &session).await;
}

#[tokio::test]
async fn tools_list_returns_json_after_initialized() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;
    initialized_notification(&app, &session).await;

    let (status, body, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})),
        Some(&session),
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["result"]["tools"][0]["name"], "echo");
}

#[tokio::test]
async fn tools_list_via_sse_streams_prime_and_response() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;
    initialized_notification(&app, &session).await;

    let (status, body, headers) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":3,"method":"tools/list"})),
        Some(&session),
        "application/json, text/event-stream",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-type").unwrap(), "text/event-stream");
    assert!(body.starts_with("id: "));
    assert!(body.contains("data: "));
    assert!(body.contains("\"echo\""));
}

#[tokio::test]
async fn tools_call_returns_content() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;
    initialized_notification(&app, &session).await;

    let (status, body, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"echo","arguments":{"text":"oi"}}})),
        Some(&session),
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["result"]["content"][0]["text"], "oi");
}

#[tokio::test]
async fn notification_without_session_is_400() {
    let (app, _http) = build_app(server_with_tool());

    let (status, _, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","method":"notifications/initialized"})),
        None,
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_terminates_session_and_allows_reinitialization() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;
    initialized_notification(&app, &session).await;

    let (status, _, _) = send(
        &app,
        Method::DELETE,
        None,
        Some(&session),
        "application/json",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(http.session_id(), None);

    let (status_old, _, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":9,"method":"tools/list"})),
        Some(&session),
        "application/json",
        None,
        None,
    )
    .await;
    assert_eq!(status_old, StatusCode::NOT_FOUND);

    let new_session = initialize(&app, &http).await;
    assert_ne!(new_session, session);
    initialized_notification(&app, &new_session).await;

    let (status, body, _) = send(
        &app,
        Method::POST,
        Some(json!({"jsonrpc":"2.0","id":10,"method":"tools/list"})),
        Some(&new_session),
        "application/json",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["result"]["tools"][0]["name"], "echo");
}

#[tokio::test]
async fn delete_without_session_is_400() {
    let (app, _http) = build_app(server_with_tool());

    let (status, _, _) = send(
        &app,
        Method::DELETE,
        None,
        None,
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn origin_policy_can_deny() {
    let http = McpHttpServer::for_server(server_with_tool())
        .origin_policy(OriginPolicy::DenyIfPresent)
        .build();
    let app = http.router();

    let (status, body, _) = send(
        &app,
        Method::POST,
        Some(initialize_body()),
        None,
        "application/json",
        Some("https://malicioso.example"),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["error"]["code"], -32600);
}

#[tokio::test]
async fn unsupported_protocol_version_is_400() {
    let (app, _http) = build_app(server_with_tool());

    let (status, _, _) = send(
        &app,
        Method::POST,
        Some(initialize_body()),
        None,
        "application/json",
        None,
        Some("1999-01-01"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unsupported_method_is_405() {
    let (app, _http) = build_app(server_with_tool());

    let (status, _, _) = send(
        &app,
        Method::PUT,
        Some(json!({})),
        None,
        "application/json",
        None,
        None,
    )
    .await;

    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn malformed_body_is_parse_error() {
    let (app, http) = build_app(server_with_tool());
    let session = initialize(&app, &http).await;

    let builder = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header("accept", "application/json")
        .header("mcp-session-id", session);
    let request = builder
        .header("content-type", "application/json")
        .body(Body::from("{nao é json"))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_str(&String::from_utf8_lossy(&bytes)).unwrap();
    assert_eq!(value["error"]["code"], -32700);
}

#[tokio::test]
async fn get_stream_requires_session() {
    let (app, _http) = build_app(server_with_tool());

    let (status_without, _, _) = send(
        &app,
        Method::GET,
        None,
        None,
        "text/event-stream",
        None,
        None,
    )
    .await;
    assert_eq!(status_without, StatusCode::BAD_REQUEST);

    let (status_wrong, _, _) = send(
        &app,
        Method::GET,
        None,
        Some("sessao-invalida"),
        "text/event-stream",
        None,
        None,
    )
    .await;
    assert_eq!(status_wrong, StatusCode::NOT_FOUND);
}
