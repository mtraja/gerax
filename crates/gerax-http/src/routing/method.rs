#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    Custom(String),
}

impl From<http::Method> for HttpMethod {
    fn from(m: http::Method) -> Self {
        match m {
            http::Method::GET => HttpMethod::Get,
            http::Method::POST => HttpMethod::Post,
            http::Method::PUT => HttpMethod::Put,
            http::Method::PATCH => HttpMethod::Patch,
            http::Method::DELETE => HttpMethod::Delete,
            http::Method::HEAD => HttpMethod::Head,
            http::Method::OPTIONS => HttpMethod::Options,
            http::Method::TRACE => HttpMethod::Trace,
            http::Method::CONNECT => HttpMethod::Connect,
            _ => HttpMethod::Custom(m.as_str().to_string()),
        }
    }
}

impl From<HttpMethod> for http::Method {
    fn from(m: HttpMethod) -> Self {
        match m {
            HttpMethod::Get => http::Method::GET,
            HttpMethod::Post => http::Method::POST,
            HttpMethod::Put => http::Method::PUT,
            HttpMethod::Patch => http::Method::PATCH,
            HttpMethod::Delete => http::Method::DELETE,
            HttpMethod::Head => http::Method::HEAD,
            HttpMethod::Options => http::Method::OPTIONS,
            HttpMethod::Trace => http::Method::TRACE,
            HttpMethod::Connect => http::Method::CONNECT,
            HttpMethod::Custom(s) => http::Method::from_bytes(s.as_bytes())
                .unwrap_or(http::Method::GET),
        }
    }
}