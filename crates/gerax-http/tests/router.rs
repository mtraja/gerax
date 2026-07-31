use gerax_http::routing::{Context, HttpMethod, Request, Response, Scope};
use gerax_http::{HttpServerError, Middleware, Next, Router, ServerResult};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct TestState;

struct TraceMiddleware {
    name: &'static str,
    trace: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait::async_trait]
impl<State: Send + Sync + 'static> Middleware<State> for TraceMiddleware {
    fn name(&self) -> &str {
        self.name
    }

    async fn handle(&self, ctx: Context<State>, next: Next<State>) -> ServerResult<Response> {
        self.trace.lock().unwrap().push(self.name);
        let response = next.call(ctx).await;
        self.trace.lock().unwrap().push(self.name);
        response
    }
}

#[tokio::test]
async fn router_returns_200_for_simple_get_route() {
    let router = Router::new().get("/ping", |_ctx| async move { Ok(Response::ok("pong")) });

    let request = Request::new(HttpMethod::Get, "/ping".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);

    let result = router.handle(ctx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, 200);
}

#[tokio::test]
async fn router_returns_200_for_simple_post_route() {
    let router = Router::new().post("/users", |_ctx| async move { Ok(Response::ok("created")) });

    let request = Request::new(HttpMethod::Post, "/users".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);

    let result = router.handle(ctx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, 200);
}

#[tokio::test]
async fn router_matches_named_parameter() {
    let router = Router::new().get("/users/:id", |ctx: Context<TestState>| async move {
        let id = ctx.params().get("id").unwrap_or("");
        Ok(Response::ok(id.to_string()))
    });

    let request = Request::new(HttpMethod::Get, "/users/42".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);

    let result = router.handle(ctx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().body, b"42");
}

#[tokio::test]
async fn router_matches_multiple_named_parameters() {
    let router = Router::new().get(
        "/users/:id/posts/:post_id",
        |ctx: Context<TestState>| async move {
            let id = ctx.params().get("id").unwrap_or("");
            let post_id = ctx.params().get("post_id").unwrap_or("");
            Ok(Response::ok(format!("{}_{}", id, post_id)))
        },
    );

    let request = Request::new(
        HttpMethod::Get,
        "/users/10/posts/99".to_string(),
        Vec::new(),
    );
    let ctx = Context::new(std::sync::Arc::new(TestState), request);

    let result = router.handle(ctx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().body, b"10_99");
}

#[tokio::test]
async fn router_returns_404_for_non_existent_route() {
    let router = Router::new().get("/ping", |_ctx| async move { Ok(Response::ok("pong")) });

    let request = Request::new(HttpMethod::Get, "/missing".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);

    let result = router.handle(ctx).await;
    assert!(result.is_err());
    match result {
        Err(HttpServerError::HandlerError(msg)) => {
            assert_eq!(msg, "Route not found");
        }
        _ => panic!("unexpected error"),
    }
}

#[tokio::test]
async fn router_returns_method_not_allowed_for_wrong_method() {
    let router = Router::new().get("/ping", |_ctx| async move { Ok(Response::ok("pong")) });

    let request = Request::new(HttpMethod::Post, "/ping".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);

    let result = router.handle(ctx).await;
    assert!(result.is_err());
    match result {
        Err(HttpServerError::HandlerError(msg)) => {
            assert_eq!(msg, "Method not allowed");
        }
        _ => panic!("unexpected error"),
    }
}

#[tokio::test]
async fn router_supports_multiple_methods_for_the_same_path() {
    let router = Router::new()
        .get("/users", |_ctx| async move { Ok(Response::ok("get")) })
        .post("/users", |_ctx| async move { Ok(Response::ok("post")) });

    for (method, expected) in [
        (HttpMethod::Get, b"get".as_slice()),
        (HttpMethod::Post, b"post".as_slice()),
    ] {
        let ctx = Context::new(
            Arc::new(TestState),
            Request::new(method, "/users".to_string(), Vec::new()),
        );
        assert_eq!(router.handle(ctx).await.unwrap().body, expected);
    }
}

#[tokio::test]
async fn router_handles_scope_with_prefix() {
    let api_scope =
        Scope::new("/api/v1").get("/items/:item_id", |ctx: Context<TestState>| async move {
            let item_id = ctx.params().get("item_id").unwrap_or("");
            Ok(Response::ok(format!("item {}", item_id)))
        });

    let router = Router::new().scope(api_scope);

    let request = Request::new(HttpMethod::Get, "/api/v1/items/123".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);

    let result = router.handle(ctx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().body, b"item 123");
}

#[tokio::test]
async fn router_handles_multiple_scopes() {
    let api_scope =
        Scope::new("/api/v1").get("/health", |_ctx| async move { Ok(Response::ok("ok")) });

    let admin_scope =
        Scope::new("/admin").get(
            "/dashboard",
            |_ctx| async move { Ok(Response::ok("dashboard")) },
        );

    let router = Router::new().scope(api_scope).scope(admin_scope);

    let request = Request::new(HttpMethod::Get, "/api/v1/health".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);
    let result = router.handle(ctx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().body, b"ok");

    let request = Request::new(HttpMethod::Get, "/admin/dashboard".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);
    let result = router.handle(ctx).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().body, b"dashboard");
}

#[tokio::test]
async fn nested_scopes_inherit_prefixes_and_middlewares() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let router = Router::new()
        .middleware(TraceMiddleware {
            name: "router",
            trace: Arc::clone(&trace),
        })
        .scope(
            Scope::new("/api")
                .middleware(TraceMiddleware {
                    name: "api",
                    trace: Arc::clone(&trace),
                })
                .scope(
                    Scope::new("/admin")
                        .middleware(TraceMiddleware {
                            name: "admin",
                            trace: Arc::clone(&trace),
                        })
                        .get("/health", |_ctx| async move { Ok(Response::ok("ok")) }),
                ),
        );

    let ctx = Context::new(
        Arc::new(TestState),
        Request::new(HttpMethod::Get, "/api/admin/health".to_string(), Vec::new()),
    );
    assert_eq!(router.handle(ctx).await.unwrap().body, b"ok");
    assert_eq!(
        trace.lock().unwrap().as_slice(),
        ["router", "api", "admin", "admin", "api", "router"]
    );
}
