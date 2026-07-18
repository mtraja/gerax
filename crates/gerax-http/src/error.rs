use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum GeraxHttpError {
    /// Falha ao vincular o servidor ao endereço/porta configurado.
    Bind(String),
    /// Falha durante a execução do servidor (após ele já estar rodando).
    Runtime(String),
    /// Falha durante o processo de encerramento (shutdown) do servidor.
    Shutdown(String),
    /// Configuração inválida ou incompleta antes da inicialização.
    Config(String),
}

impl fmt::Display for GeraxHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(msg) => write!(f, "falha ao iniciar (bind): {msg}"),
            Self::Runtime(msg) => write!(f, "falha em tempo de execução: {msg}"),
            Self::Shutdown(msg) => write!(f, "falha ao encerrar: {msg}"),
            Self::Config(msg) => write!(f, "configuração inválida: {msg}"),
        }
    }
}

impl StdError for GeraxHttpError {}
