use serde::Deserialize;

/// Configuração do servidor Actix.
#[derive(Debug, Clone, Deserialize)]
pub struct ActixConfig {
    /// Host de bind do servidor.
    pub host: String,
    /// Porta de bind do servidor.
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
