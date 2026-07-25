use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ActixConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ActixConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}
