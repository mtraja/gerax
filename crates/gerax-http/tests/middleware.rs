use std::sync::atomic::{AtomicBool, Ordering};

use gerax_http::middleware::{Middleware, Next};
use gerax_http::routing::{Context, HttpMethod, Request, Response, Route};
use gerax_http::{HttpServerError, ServerResult};

#[derive(Clone)]
struct TestState;

async fn ok_handler(_ctx: Context<TestState>) -> ServerResult<Response> {
    Ok(Response::ok("handler"))
}

async fn error_handler(_ctx: Context<TestState>) -> ServerResult<Response> {
    Err(HttpServerError::HandlerError("boom".to_string()))
}

async fn hello_handler(_ctx: Context<TestState>) -> ServerResult<Response> {
    Ok(Response::ok("world"))
}

struct OrderMiddleware {
    name: &'static str,
    order: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
}

#[async_trait::async_trait]
impl<State: Send + Sync + 'static> Middleware<State> for OrderMiddleware {
    fn name(&self) -> &str {
        self.name
    }

    async fn handle(&self, ctx: Context<State>, next: Next<State>) -> ServerResult<Response> {
        self.order.lock().unwrap().push(self.name);
        let result = next.call(ctx).await;
        self.order.lock().unwrap().push(self.name);
        result
    }
}

#[tokio::test]
async fn middleware_executes_in_registration_order() {
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut route = Route::new(HttpMethod::Get, "/hello", hello_handler);
    route = route.middleware(OrderMiddleware {
        name: "M1",
        order: std::sync::Arc::clone(&order),
    });
    route = route.middleware(OrderMiddleware {
        name: "M2",
        order: std::sync::Arc::clone(&order),
    });

    let request = Request::new(HttpMethod::Get, "/hello".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);
    let result = route.execute(ctx).await;

    assert!(result.is_ok());
    let recorded = order.lock().unwrap();
    assert_eq!(recorded.as_slice(), &["M1", "M2", "M2", "M1"]);
}

#[tokio::test]
async fn middleware_calls_next_and_continues_to_handler() {
    let handler_invoked = std::sync::Arc::new(AtomicBool::new(false));

    struct PassMiddleware {
        handler_invoked: std::sync::Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl<State: Send + Sync + 'static> Middleware<State> for PassMiddleware {
        fn name(&self) -> &str {
            "pass"
        }

        async fn handle(&self, ctx: Context<State>, next: Next<State>) -> ServerResult<Response> {
            let result = next.call(ctx).await;
            if result.is_ok() {
                self.handler_invoked.store(true, Ordering::SeqCst);
            }
            result
        }
    }

    let mut route = Route::new(HttpMethod::Get, "/hello", ok_handler);
    route = route.middleware(PassMiddleware {
        handler_invoked: std::sync::Arc::clone(&handler_invoked),
    });

    let request = Request::new(HttpMethod::Get, "/hello".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);
    route.execute(ctx).await.unwrap();

    assert!(handler_invoked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn middleware_short_circuits_without_calling_next() {
    let middleware_invoked = std::sync::Arc::new(AtomicBool::new(false));

    struct ShortCircuitMiddleware {
        middleware_invoked: std::sync::Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl<State: Send + Sync + 'static> Middleware<State> for ShortCircuitMiddleware {
        fn name(&self) -> &str {
            "short-circuit"
        }

        async fn handle(&self, _ctx: Context<State>, _next: Next<State>) -> ServerResult<Response> {
            self.middleware_invoked.store(true, Ordering::SeqCst);
            Ok(Response::ok("short-circuited"))
        }
    }

    let mut route = Route::new(HttpMethod::Get, "/hello", hello_handler);
    route = route.middleware(ShortCircuitMiddleware {
        middleware_invoked: std::sync::Arc::clone(&middleware_invoked),
    });

    let request = Request::new(HttpMethod::Get, "/hello".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);
    let result = route.execute(ctx).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().body, b"short-circuited");
    assert!(middleware_invoked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn middleware_intercepts_error_from_handler() {
    struct ErrorCatcher {
        last_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    }

    #[async_trait::async_trait]
    impl<State: Send + Sync + 'static> Middleware<State> for ErrorCatcher {
        fn name(&self) -> &str {
            "error-catcher"
        }

        async fn handle(&self, ctx: Context<State>, next: Next<State>) -> ServerResult<Response> {
            let result = next.call(ctx).await;
            if let Err(e) = result {
                let msg = e.to_string();
                self.last_error.lock().unwrap().replace(msg);
                return Ok(Response::ok("handled-error"));
            }
            Ok(result.unwrap())
        }
    }

    let last_error = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut route = Route::new(HttpMethod::Get, "/hello", error_handler);
    route = route.middleware(ErrorCatcher {
        last_error: std::sync::Arc::clone(&last_error),
    });

    let request = Request::new(HttpMethod::Get, "/hello".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);
    let result = route.execute(ctx).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().body, b"handled-error");
    assert_eq!(
        last_error.lock().unwrap().as_deref(),
        Some("Erro no handler: boom")
    );
}

#[tokio::test]
async fn middleware_returns_error_directly() {
    struct AbortMiddleware;

    #[async_trait::async_trait]
    impl<State: Send + Sync + 'static> Middleware<State> for AbortMiddleware {
        fn name(&self) -> &str {
            "abort"
        }

        async fn handle(&self, _ctx: Context<State>, _next: Next<State>) -> ServerResult<Response> {
            Err(HttpServerError::HandlerError("forbidden".to_string()))
        }
    }

    let mut route = Route::new(HttpMethod::Get, "/hello", ok_handler);
    route = route.middleware(AbortMiddleware);

    let request = Request::new(HttpMethod::Get, "/hello".to_string(), Vec::new());
    let ctx = Context::new(std::sync::Arc::new(TestState), request);
    let result = route.execute(ctx).await;

    assert!(matches!(result, Err(HttpServerError::HandlerError(ref msg)) if msg == "forbidden"));
}
