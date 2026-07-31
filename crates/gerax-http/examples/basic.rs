use gerax_http::routing::{Context, HttpMethod, Request, Response, Router, Scope};
use gerax_http::middleware::{Middleware, Next};
use gerax_http::{HttpServerError, ServerResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
struct AppState;

#[derive(Debug, Deserialize, Serialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct LoginQuery {
    redirect: Option<String>,
}

struct Logger;

#[async_trait::async_trait]
impl<State> Middleware<State> for Logger
where
    State: Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "logger"
    }

    async fn handle(
        &self,
        ctx: Context<State>,
        next: Next<State>,
    ) -> ServerResult<Response> {
        println!(
            "[Logger] {:?} {}",
            ctx.request().method(),
            ctx.request().path()
        );
        next.call(ctx).await
    }
}

async fn health(_ctx: Context<AppState>) -> ServerResult<Response> {
    Ok(Response::ok(b"ok".to_vec()))
}

async fn get_user(ctx: Context<AppState>) -> ServerResult<Response> {
    let user_id = ctx
        .params()
        .get("id")
        .ok_or_else(|| HttpServerError::HandlerError("missing id".into()))?;
    let _id: u64 = user_id
        .parse()
        .map_err(|e: std::num::ParseIntError| HttpServerError::HandlerError(e.to_string()))?;
    Ok(Response::ok(b"ok".to_vec()))
}

async fn login(ctx: Context<AppState>) -> ServerResult<Response> {
    let query = ctx.request().query.clone();
    let q: LoginQuery = serde_urlencoded::from_str(&query)
        .map_err(|e| HttpServerError::HandlerError(e.to_string()))?;
    let redirect = q.redirect.unwrap_or_else(|| "/".to_string());
    Ok(Response::ok(format!("redirect = {redirect}")))
}

async fn create_user(ctx: Context<AppState>) -> ServerResult<Response> {
    let payload: CreateUser = serde_json::from_slice(&ctx.request().body)
        .map_err(|e| HttpServerError::HandlerError(e.to_string()))?;
    let body = serde_json::to_vec(&payload).unwrap();
    Ok(Response {
        status: 201,
        body,
    })
}

fn build_router() -> Router<AppState> {
    Router::new()
        .middleware(Logger)
        .get("/health", health)
        .scope(
            Scope::new("/api")
                .get("/login", login)
                .post("/users", create_user),
        )
        .scope(Scope::new("/api/v1").get("/users/:id", get_user))
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState);
    let router = build_router();

    println!("=== GET /health ===");
    let req = Request::new(HttpMethod::Get, "/health".to_string(), Vec::new());
    let ctx = Context::new(Arc::clone(&state), req);
    match router.handle(ctx).await {
        Ok(r) => println!("{} -> {}", r.status, String::from_utf8_lossy(&r.body)),
        Err(e) => println!("error: {e:?}"),
    }

    println!("\n=== GET /api/v1/users/42 ===");
    let req = Request::new(HttpMethod::Get, "/api/v1/users/42".to_string(), Vec::new());
    let ctx = Context::new(Arc::clone(&state), req);
    match router.handle(ctx).await {
        Ok(r) => println!("{} -> {}", r.status, String::from_utf8_lossy(&r.body)),
        Err(e) => println!("error: {e:?}"),
    }

    println!("\n=== GET /api/login?redirect=/dashboard ===");
    let mut req = Request::new(HttpMethod::Get, "/api/login".to_string(), Vec::new());
    req.query = "redirect=%2Fdashboard".to_string();
    let ctx = Context::new(Arc::clone(&state), req);
    match router.handle(ctx).await {
        Ok(r) => println!("{} -> {}", r.status, String::from_utf8_lossy(&r.body)),
        Err(e) => println!("error: {e:?}"),
    }

    println!("\n=== POST /api/users ===");
    let payload = CreateUser {
        name: "Ada".into(),
        email: "ada@example.com".into(),
    };
    let body = serde_json::to_vec(&payload).unwrap();
    let req = Request::new(HttpMethod::Post, "/api/users".to_string(), body);
    let ctx = Context::new(Arc::clone(&state), req);
    match router.handle(ctx).await {
        Ok(r) => println!("{} -> {}", r.status, String::from_utf8_lossy(&r.body)),
        Err(e) => println!("error: {e:?}"),
    }
}
