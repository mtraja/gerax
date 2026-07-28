use serde::Deserialize;

/// Configuração do servidor Actix.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Host de bind do servidor.
    pub url: String,
    /// Porta de bind do servidor.
    pub name: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://user:password@host:port/name_db".to_string(),
            name: "demo_db".to_string(),
        }
    }
}
