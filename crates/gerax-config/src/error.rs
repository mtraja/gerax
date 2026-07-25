use std::io;

use thiserror::Error;

/// Resultado padrão do gerax-config.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Representa os erros em Config
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Erro de IO
    #[error("IO error reading '{path}': {source}")]
    Io {
        /// caminho do erro
        path: String,

        /// fonte do erro
        #[source]
        source: io::Error,
    },

    #[cfg(feature = "toml")]
    #[error("TOML parse error in '{path}': {source}")]
    Toml {
        path: String,

        #[source]
        source: toml::de::Error,
    },

    #[cfg(feature = "yaml")]
    #[error("YAML parse error in '{path}': {source}")]
    Yaml {
        path: String,

        #[source]
        source: serde_yaml::Error,
    },

    #[cfg(feature = "json")]
    #[error("JSON parse error in '{path}': {source}")]
    Json {
        path: String,

        #[source]
        source: serde_json::Error,
    },

    #[error("environment error loading '{path}': {source}")]
    Env {
        path: String,

        #[source]
        source: dotenvy::Error,
    },

    #[error("configuration deserialize error: {0}")]
    Deserialize(String),

    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("missing configuration value: {0}")]
    Missing(String),
}
