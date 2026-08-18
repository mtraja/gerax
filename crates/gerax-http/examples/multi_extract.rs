use gerax_http::routing::{Context, HttpMethod, Request, Response, Router, State, Json};
use gerax_http::{ServerResult, handler};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
struct AppState;

#[derive(Debug, Deserialize, Serialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[handler]
async fn health<S>(State(_state): State<S>) -> ServerResult<Response> {
    Ok(Response::ok(b"ok".to_vec()))
}

#[handler]
async fn create_user<S>(Json(payload): Json<CreateUserInput>, State(_state): State<S>) -> ServerResult<Response> {
    let body = serde_json::to_vec(&payload).unwrap();
    Ok(Response {
        status: 201,
        body,
    })
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState);
    let router = Router::new()
        .get("/health", HealthHandler)
        .post("/users", CreateUserHandler);

    println!("=== GET /health ===");
    let req = Request::new(HttpMethod::Get, "/health".to_string(), Vec::new());
    let ctx = Context::new(Arc::clone(&state), req);
    match router.handle(ctx).await {
        Ok(r) => println!("{} -> {}", r.status, String::from_utf8_lossy(&r.body)),
        Err(e) => println!("error: {e:?}"),
    }

    println!("\n=== POST /users ===");
    let payload = CreateUserInput {
        name: "Ada".into(),
        email: "ada@example.com".into(),
    };
    let body = serde_json::to_vec(&payload).unwrap();
    let req = Request::new(HttpMethod::Post, "/users".to_string(), body);
    let ctx = Context::new(Arc::clone(&state), req);
    match router.handle(ctx).await {
        Ok(r) => println!("{} -> {}", r.status, String::from_utf8_lossy(&r.body)),
        Err(e) => println!("error: {e:?}"),
    }
}
