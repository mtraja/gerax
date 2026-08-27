#[derive(Debug, Clone, Default)]
pub struct CorsConfig {
    pub allow_any_origin: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub max_age: Option<u64>,
}
/*
impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allow_any_origin: false,
            allowed_origins: Vec::new(),
            allowed_methods: vec![
                "GET".into(),
                "POST".into(),
                "PUT".into(),
                "PATCH".into(),
                "DELETE".into(),
                "OPTIONS".into(),
            ],
            allowed_headers: vec![
                "Content-Type".into(),
                "Authorization".into(),
            ],
            max_age: Some(3600),
        }
    }
}
*/


impl CorsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_any_origin(mut self) -> Self {
        self.allow_any_origin = true;
        self
    }

    pub fn allowed_origin(mut self, origin: impl Into<String>) -> Self {
        self.allowed_origins.push(origin.into());
        self
    }

    pub fn allowed_method(mut self, method: impl Into<String>) -> Self {
        self.allowed_methods.push(method.into());
        self
    }

    pub fn allowed_all_methods(mut self) -> Self {
        self.allowed_methods =  vec![
                "GET".into(),
                "POST".into(),
                "PUT".into(),
                "PATCH".into(),
                "DELETE".into(),
                "OPTIONS".into(),
            ];
        self
    }

    pub fn allowed_header(mut self, header: impl Into<String>) -> Self {
        self.allowed_headers.push(header.into());
        self
    }

    pub fn max_age(mut self, seconds: u64) -> Self {
        self.max_age = Some(seconds);
        self
    }
}
