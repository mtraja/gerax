use gerax_config::ConfigBuilder;
use gerax_http::ServerConfig;
use serde::Deserialize;

use crate::AppResult;

/// Configuração agregada de uma aplicação Gerax.
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// Configuração do servidor HTTP.
    #[serde(default)]
    pub server: ServerConfig,

    /// Configuração padrão do banco de dados da aplicação.
    #[cfg(feature = "db")]
    #[serde(default)]
    pub database: gerax_db::DatabaseConfig,
}

impl AppConfig {
    /// Carrega a configuração a partir das fontes registradas no builder.
    pub fn load(builder: ConfigBuilder) -> AppResult<Self> {
        Ok(builder.build()?)
    }
}
