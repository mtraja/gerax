use std::sync::Arc;

use actix_web::{
    App, HttpResponse, HttpServer as ActixWebHttpServer, http::{Method, StatusCode}, web::{self, method},
};
use async_trait::async_trait;

use gerax_http::{
    HttpServer, HttpServerError, ServerResult, routing::{Context, Extensions, HttpMethod, PathParams, Request, Router},
};



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
    pub middlewares: Vec<Arc<dyn gerax_http::Middleware>>,
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

        actix_web::rt::System::new().block_on(async move {
            let server = ActixWebHttpServer::new(move || {
                let mut app = App::new().app_data(web::Data::new(state.clone()));

                if let Some(ref router) = router {
                    let router = router.clone();

                    for route in router.routes() {
                        let handler = route.handler().clone();
                        let path = route.path().to_string();
                        let method = method_to_actix(route.method());

                        app =
                            app.route(
                                &path,
                                match method {
                                    Method::GET => web::get()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::POST => web::post()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::PUT => web::put()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::PATCH => web::patch()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::DELETE => web::delete()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::HEAD => web::head()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    _ => web::get()
                                        .to(move |req| route_handler(req, handler.clone())),
                                },
                            );
                    }

                    for scope in router.scopes() {
                        let mut scope_app = actix_web::Scope::new(scope.prefix());

                        for route in scope.routes() {
                            let handler = route.handler().clone();
                            let path = route.path().to_string();
                            let method = method_to_actix(route.method());

                            scope_app = scope_app.route(
                                &path,
                                match method {
                                    Method::GET => web::get()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::POST => web::post()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::PUT => web::put()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::PATCH => web::patch()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::DELETE => web::delete()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    Method::HEAD => web::head()
                                        .to(move |req| route_handler(req, handler.clone())),
                                    _ => web::get()
                                        .to(move |req| route_handler(req, handler.clone())),
                                },
                            );
                        }

                        app = app.service(scope_app);
                    }
                }

                app
            });

            server
                .bind(format!("{}:{}", host, port))
                .map_err(|e| HttpServerError::InitializationFailed(e.to_string()))?
                .run()
                .await
                .map_err(|e| HttpServerError::RuntimeError(e.to_string()))
        })
    }
}

async fn route_handler<S>(
    req: actix_web::HttpRequest,
    handler: Arc<dyn gerax_http::routing::Handler<S>>,
) -> HttpResponse
where
    S: Send + Sync + 'static,
{
    //let data = req
    //    .app_data::<web::Data<Arc<S>>>()
    //    .expect("state not stored")
    //    .get_ref()
    //    .clone();

    let data = match req.app_data::<web::Data<Arc<S>>>() {
        Some(data) => data.get_ref().clone(),
        None => {
            return HttpResponse::InternalServerError()
                .body("Application state not configured");
        }
    };

    let method = converter_method(req.method().clone());
    let request = Request::new(method, req.path().to_string(), Vec::new());
    let context = Context::new(data, request);

    let response = handler.call(context).await;
    HttpResponse::build(status_from_u16(response.status)).body(response.body)
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
        _ => todo!(),
    }
}

fn status_from_u16(status: u16) -> StatusCode {
    StatusCode::from_u16(status).unwrap_or(StatusCode::OK)
}
