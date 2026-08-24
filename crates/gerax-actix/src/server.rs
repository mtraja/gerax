use std::sync::Arc;

use actix_web::{
    App, HttpResponse, HttpServer as ActixWebHttpServer, http::{Method, StatusCode}, web,
};
use async_trait::async_trait;

use gerax_http::{
    HttpServer, HttpServerError, ServerResult, routing::{Context, HttpMethod, Request, Route, Router},
};



fn extend_route_middlewares<S>(
    mut route: Route<S>,
    middlewares: &[Arc<dyn gerax_http::Middleware<S>>],
) -> Route<S>
where
    S: Send + Sync + 'static,
{
    let mut all_middlewares = route.middlewares().to_vec();
    all_middlewares.extend(middlewares.iter().map(Arc::clone));
    route.set_middlewares(all_middlewares);
    route
}

/// Representa um servidor HTTP implementado com Actix Web.
pub struct ActixHttpServer<S> {
    /// Estado compartilhado da aplicação.
    pub state: Arc<S>,
    /// Rotas configuradas.
    pub router: Option<Arc<Router<S>>>,
    /// Host de bind do servidor.
    pub host: String,
    /// Porta de bind do servidor.
    pub port: u16,
    /// Middlewares registrados.
    pub middlewares: Vec<Arc<dyn gerax_http::Middleware<S>>>,
}

#[async_trait]
impl<S> HttpServer for ActixHttpServer<S>
where
    S: Send + Sync + 'static,
{
    /// Inicia o servidor Actix e aguarda encerramento ou erro.
    async fn run(self) -> ServerResult {
        let host = self.host;
        let port = self.port;
        let state = self.state;
        let router = self.router;
        let server_middlewares = self.middlewares;

        let result = tokio::task::spawn_blocking(move || {
            actix_web::rt::System::new().block_on(async move {
                ActixWebHttpServer::new(move || {
                    let mut app = App::new().app_data(web::Data::new(state.clone()));

                    if let Some(ref router) = router {
                        let router = router.clone();

                        for route in router.routes() {
                            let route = extend_route_middlewares(route.clone(), router.middlewares());
                            let route = extend_route_middlewares(route, &server_middlewares);
                            let path = route.path().to_string();
                            let method = method_to_actix(route.method());
                            let route_arc = Arc::new(route);

                        app =
                            app.route(
                                &path,
                                match method {
                                    Method::GET => web::get().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::POST => web::post().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::PUT => web::put().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::PATCH => web::patch().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::DELETE => web::delete().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::HEAD => web::head().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    _ => web::get().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                },
                            );
                        }

                        for scope in router.scopes() {
                            let mut scope_app = actix_web::Scope::new(scope.prefix());

                            for route in scope.routes() {
                                let route = extend_route_middlewares(route.clone(), scope.middlewares());
                                let route = extend_route_middlewares(route, router.middlewares());
                                let route = extend_route_middlewares(route, &server_middlewares);
                                let path = route.path().to_string();
                                let method = method_to_actix(route.method());
                                let route_arc = Arc::new(route);

                            scope_app = scope_app.route(
                                &path,
                                match method {
                                    Method::GET => web::get().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::POST => web::post().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::PUT => web::put().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::PATCH => web::patch().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::DELETE => web::delete().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    Method::HEAD => web::head().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                    _ => web::get().to(move |req, body| {
                                        route_handler(req, body, route_arc.clone())
                                    }),
                                },
                            );
                            }

                            app = app.service(scope_app);
                        }
                    }

                    app
                })
                .bind(format!("{}:{}", host, port))
                .map_err(|e| HttpServerError::InitializationFailed(e.to_string()))?
                .run()
                .await
                .map_err(|e| HttpServerError::RuntimeError(e.to_string()))
            })
        })
        .await
        .map_err(|e| HttpServerError::RuntimeError(e.to_string()))??;

        Ok(result)
    }
}

async fn route_handler<S>(
    req: actix_web::HttpRequest,
    body: web::Bytes,
    route: Arc<Route<S>>,
) -> HttpResponse
where
    S: Send + Sync + 'static,
{
    let data = match req.app_data::<web::Data<Arc<S>>>() {
        Some(data) => data.get_ref().clone(),
        None => {
            return HttpResponse::InternalServerError()
                .body("Application state not configured");
        }
    };

    let method = converter_method(req.method().clone());
    let mut request = Request::new(method, req.path().to_string(), body.to_vec());

    for (name, value) in req.headers() {
        let value = match value.to_str() {
            Ok(value) => value,
            Err(_) => {
                return HttpResponse::BadRequest().body("Invalid HTTP header value");
            }
        };

        request.headers.insert(name.as_str(), value);
    }

    let context = Context::new(data, request);

    let response = route.execute(context).await;
    match response {
        Ok(response) => HttpResponse::build(status_from_u16(response.status)).body(response.body),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

fn method_to_actix(method: HttpMethod) -> Method {
    match method {
        HttpMethod::Get => Method::GET,
        HttpMethod::Post => Method::POST,
        HttpMethod::Put => Method::PUT,
        HttpMethod::Patch => Method::PATCH,
        HttpMethod::Delete => Method::DELETE,
        HttpMethod::Head => Method::HEAD,
        HttpMethod::Options => Method::OPTIONS,
        HttpMethod::Trace | HttpMethod::Connect | HttpMethod::Custom(_) => Method::GET,
    }
}

fn converter_method(method: Method) -> HttpMethod {
    match method {
        Method::GET => HttpMethod::Get,
        Method::POST => HttpMethod::Post,
        Method::PUT => HttpMethod::Put,
        Method::PATCH => HttpMethod::Patch,
        Method::DELETE => HttpMethod::Delete,
        Method::HEAD => HttpMethod::Head,
        Method::OPTIONS => HttpMethod::Options,
        Method::TRACE => HttpMethod::Trace,
        Method::CONNECT => HttpMethod::Connect,
        _ => HttpMethod::Custom(method.as_str().to_string()),
    }
}

fn status_from_u16(status: u16) -> StatusCode {
    StatusCode::from_u16(status).unwrap_or(StatusCode::OK)
}
