use serde::Deserialize;

/// Configuração do servidor Actix.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Host de bind do servidor.
    pub url: String,
    /// Porta de bind do servidor.
    pub database: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://admin:admin@host:5432/demo_db".to_string(),
            database: "demo_db".to_string(),
        }
    }
}
