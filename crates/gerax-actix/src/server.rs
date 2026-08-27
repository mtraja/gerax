use std::{str::FromStr, sync::Arc};

use actix_cors::Cors;
use actix_web::{
    App, HttpResponse, HttpServer as ActixWebHttpServer,
    http::{Method, StatusCode},
    web,
};
use async_trait::async_trait;

use gerax_http::{
    HttpServer, HttpServerError, ServerResult,
    routing::{Context, HttpMethod, Request, Route, Router},
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

/// Converte a sintaxe de parâmetros do roteador Gerax (`:id`) para a
/// sintaxe que o Actix Web reconhece (`{id}`).
fn actix_path(path: &str) -> String {
    path.split('/')
        .map(|segment| match segment.strip_prefix(':') {
            Some(parameter) if !parameter.is_empty() => format!("{{{parameter}}}"),
            _ => segment.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn build_cors(config: Option<&gerax_http::CorsConfig>) -> Cors {
    let Some(config) = config else {
        return Cors::default();
    };

    let mut cors = Cors::default();

    if config.allow_any_origin {
        cors = cors.allow_any_origin();
    }

    for origin in &config.allowed_origins {
        cors = cors.allowed_origin(origin);
    }

    if config.allowed_methods.is_empty() {
        cors = cors.allow_any_method();
    } else {
        let methods: Vec<Method> = config
            .allowed_methods
            .iter()
            .filter_map(|method| Method::from_str(method).ok())
            .collect();
        cors = cors.allowed_methods(methods);
    }

    if config.allowed_headers.is_empty() {
        cors = cors.allow_any_header();
    } else {
        let headers: Vec<&str> = config.allowed_headers.iter().map(String::as_str).collect();
        cors = cors.allowed_headers(headers);
    }

    if let Some(max_age) = config.max_age {
        cors = cors.max_age(max_age as usize);
    }

    cors
}

pub struct ActixHttpServer<S> {
    pub state: Arc<S>,
    pub router: Option<Arc<Router<S>>>,
    pub host: String,
    pub port: u16,
    pub middlewares: Vec<Arc<dyn gerax_http::Middleware<S>>>,
    pub cors_config: Option<gerax_http::CorsConfig>,
}

#[async_trait]
impl<S> HttpServer for ActixHttpServer<S>
where
    S: Send + Sync + 'static,
{
    async fn run(self) -> ServerResult {
        let host = self.host;
        let port = self.port;
        let state = self.state;
        let router = self.router;
        let server_middlewares = self.middlewares;
        let cors_config = self.cors_config;

        let result = tokio::task::spawn_blocking(move || {
            actix_web::rt::System::new().block_on(async move {
                ActixWebHttpServer::new(move || {
                    let mut app = App::new()
                        .app_data(web::Data::new(state.clone()))
                        .wrap(build_cors(cors_config.as_ref()));

                    if let Some(ref router) = router {
                        let router = router.clone();

                        for route in router.routes() {
                            let route =
                                extend_route_middlewares(route.clone(), router.middlewares());
                            let route = extend_route_middlewares(route, &server_middlewares);
                            let path = actix_path(route.path());
                            let method = method_to_actix(route.method());
                            let route_arc = Arc::new(route);

                            app = app.route(
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
                                let route =
                                    extend_route_middlewares(route.clone(), scope.middlewares());
                                let route = extend_route_middlewares(route, router.middlewares());
                                let route = extend_route_middlewares(route, &server_middlewares);
                                let path = actix_path(route.path());
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
            return HttpResponse::InternalServerError().body("Application state not configured");
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

    let mut context = Context::new(data, request);
    for (name, value) in req.match_info().iter() {
        context
            .params_mut()
            .insert(name.to_owned(), value.to_owned());
    }

    let response = route.execute(context).await;
    match response {
        Ok(response) => {
            let mut builder = HttpResponse::build(status_from_u16(response.status));
            for (name, value) in response.headers.iter() {
                builder.insert_header((name.as_str(), value.as_str()));
            }
            builder.body(response.body)
        }
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

#[cfg(test)]
mod tests {
    use super::actix_path;

    #[test]
    fn converts_gerax_path_parameters_to_actix_syntax() {
        assert_eq!(actix_path("/alunos/:id"), "/alunos/{id}");
        assert_eq!(
            actix_path("/turmas/:turma_id/alunos/:aluno_id"),
            "/turmas/{turma_id}/alunos/{aluno_id}"
        );
    }
}
