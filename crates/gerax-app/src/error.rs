use thiserror::Error;

use gerax_http::HttpServerError;

/// Erros produzidos durante o bootstrap ou a execução da aplicação.
#[derive(Debug, Error)]
pub enum AppError {
    /// Falha na construção ou execução do servidor HTTP.
    #[error(transparent)]
    Http(#[from] HttpServerError),

    /// Falha ao carregar a configuração da aplicação.
    #[cfg(feature = "config")]
    #[error(transparent)]
    Config(#[from] gerax_config::ConfigError),

    /// Falha ao conectar ou acessar o banco de dados.
    #[cfg(feature = "db")]
    #[error(transparent)]
    Database(#[from] gerax_db::DbError),
}

/// Resultado usado pela API pública de `gerax-app`.
pub type AppResult<T = ()> = Result<T, AppError>;
