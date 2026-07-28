use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpServerError {
    #[error("Falha ao iniciar o servidor: {0}")]
    InitializationFailed(String),

    #[error("Erro durante a execução do servidor: {0}")]
    RuntimeError(String),

    #[error("Erro de configuração: {0}")]
    ConfigurationError(String),

    #[error("Erro no handler: {0}")]
    HandlerError(String),
}

pub type ServerResult<T = ()> = Result<T, HttpServerError>;
