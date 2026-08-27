use std::str::FromStr;

use actix_cors::Cors;
use actix_web::http::Method;
use gerax_http::{
    middleware::{Middleware, Next},
    routing::Response,
    CorsConfig, ServerResult,
};

#[derive(Clone)]
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self::new(CorsConfig::new().allow_any_origin())
    }

    pub fn to_actix_cors(&self) -> Cors {
        let mut cors = Cors::default();

        if self.config.allow_any_origin {
            cors = cors.allow_any_origin();
        }

        for origin in &self.config.allowed_origins {
            cors = cors.allowed_origin(origin);
        }

        if self.config.allowed_methods.is_empty() {
            cors = cors.allow_any_method();
        } else {
            let methods: Vec<Method> = self
                .config
                .allowed_methods
                .iter()
                .filter_map(|m| Method::from_str(m).ok())
                .collect();
            cors = cors.allowed_methods(methods);
        }

        if self.config.allowed_headers.is_empty() {
            cors = cors.allow_any_header();
        } else {
            let headers: Vec<&str> = self.config.allowed_headers.iter().map(|h| h.as_str()).collect();
            cors = cors.allowed_headers(headers);
        }

        if let Some(max_age) = self.config.max_age {
            cors = cors.max_age(max_age as usize);
        }

        cors
    }
}

#[async_trait::async_trait]
impl<S> Middleware<S> for CorsMiddleware
where
    S: Send + Sync + 'static,
{
    fn name(&self) -> &str {
        "cors"
    }

    async fn handle(
        &self,
        ctx: gerax_http::routing::Context<S>,
        next: Next<S>,
    ) -> ServerResult<Response> {
        let mut response = next.call(ctx).await?;

        if self.config.allow_any_origin || !self.config.allowed_origins.is_empty() {
            response.headers.insert("access-control-allow-origin", "*");
        }

        let methods = if self.config.allowed_methods.is_empty() {
            "GET, POST, PUT, DELETE, OPTIONS, PATCH, HEAD"
        } else {
            &self.config.allowed_methods.join(", ")
        };
        response.headers.insert("access-control-allow-methods", methods);

        let headers = if self.config.allowed_headers.is_empty() {
            "Content-Type, Authorization"
        } else {
            &self.config.allowed_headers.join(", ")
        };
        response.headers.insert("access-control-allow-headers", headers);

        if let Some(max_age) = self.config.max_age {
            response.headers.insert("access-control-max-age", max_age.to_string());
        }

        Ok(response)
    }
}
