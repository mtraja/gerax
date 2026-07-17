use actix_web::{HttpServer, web, App, HttpResponse as ActixHttpResponse};
use gerax_http::{HttpError, HttpServerTrait, RouteConfigurator, RouteConfig, Route, HttpMethod, HttpResponse};
use std::sync::Arc;

/// Wrapper that makes RouteConfigurator cloneable
#[derive(Clone)]
struct RouteConfigurerWrapper {
    routes: Vec<Route>,
}

impl RouteConfigurerWrapper {
    fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }
}

impl RouteConfigurator<()> for RouteConfigurerWrapper {
    fn configure(&self, _state: ()) -> Vec<Route> {
        self.routes.clone()
    }
}

pub struct ActixServer {
    host: String,
    port: u16,
    workers: usize,
    route_configurer: Option<RouteConfigurerWrapper>,
}

impl ActixServer {
    pub fn new() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 1,
            route_configurer: None,
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    pub fn routes<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut RouteConfig) + Send + Sync + 'static,
    {
        let mut config = RouteConfig::new();
        f(&mut config);
        
        self.route_configurer = Some(RouteConfigurerWrapper::new(config.routes().to_vec()));
        self
    }
}

impl Default for ActixServer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Send + Sync + Clone + 'static> HttpServerTrait<S> for ActixServer {
    async fn start(&self, state: S) -> Result<(), HttpError> {
        let addr = format!("{}:{}", self.host, self.port);
        let state = Arc::new(state);
        let route_configurer = self.route_configurer.clone();

        let server = HttpServer::new(move || {
            let state = state.clone();
            let mut app = App::new().app_data(web::Data::new(state));

            if let Some(configurer) = &route_configurer {
                let routes = configurer.configure(());
                for route in routes {
                    let handler = route.handler.clone();
                    app = match route.method {
                        HttpMethod::Get => app.route(&route.path, web::get().to(move || {
                            let handler = handler.clone();
                            async move {
                                match handler() {
                                    Ok(resp) => ActixHttpResponse::Ok().body(resp.body),
                                    Err(e) => ActixHttpResponse::InternalServerError().body(e.to_string()),
                                }
                            }
                        })),
                        HttpMethod::Post => app.route(&route.path, web::post().to(move || {
                            let handler = handler.clone();
                            async move {
                                match handler() {
                                    Ok(resp) => ActixHttpResponse::Ok().body(resp.body),
                                    Err(e) => ActixHttpResponse::InternalServerError().body(e.to_string()),
                                }
                            }
                        })),
                        HttpMethod::Put => app.route(&route.path, web::put().to(move || {
                            let handler = handler.clone();
                            async move {
                                match handler() {
                                    Ok(resp) => ActixHttpResponse::Ok().body(resp.body),
                                    Err(e) => ActixHttpResponse::InternalServerError().body(e.to_string()),
                                }
                            }
                        })),
                        HttpMethod::Delete => app.route(&route.path, web::delete().to(move || {
                            let handler = handler.clone();
                            async move {
                                match handler() {
                                    Ok(resp) => ActixHttpResponse::Ok().body(resp.body),
                                    Err(e) => ActixHttpResponse::InternalServerError().body(e.to_string()),
                                }
                            }
                        })),
                        HttpMethod::Patch => app.route(&route.path, web::patch().to(move || {
                            let handler = handler.clone();
                            async move {
                                match handler() {
                                    Ok(resp) => ActixHttpResponse::Ok().body(resp.body),
                                    Err(e) => ActixHttpResponse::InternalServerError().body(e.to_string()),
                                }
                            }
                        })),
                    };
                }
            }

            app
        })
        .bind(&addr)
        .map_err(|e| HttpError::StartError(e.to_string()))?
        .workers(self.workers)
        .run();

        tokio::spawn(async move {
            let _ = server.await;
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_http::{RouteConfig, HttpMethod, HttpResponse};

    #[derive(Debug, Clone, Default)]
    struct AppState {
        name: String,
    }

    #[test]
    fn test_actix_server_builder() {
        let server = ActixServer::new()
            .host("127.0.0.1")
            .port(3000)
            .workers(2);
        assert_eq!(server.port, 3000);
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.workers, 2);
    }

    #[test]
    fn test_routes_config() {
        let server = ActixServer::new().routes(|config| {
            config.route(HttpMethod::Get, "/health", || Ok(HttpResponse::ok("OK")));
            config.route(HttpMethod::Get, "/users", || Ok(HttpResponse::ok("Users")));
        });
        
        assert!(server.route_configurer.is_some());
    }
}