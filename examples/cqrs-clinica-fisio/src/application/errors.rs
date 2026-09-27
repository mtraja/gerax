//! Erros da camada de aplicação.
//!
//! As portas e os handlers falam a "linguagem da aplicação"
//! (`ApplicationError`), mantendo o núcleo desacoplado do `gerax-cqrs`
//! e das bibliotecas de infraestrutura.

use std::fmt;

use gerax_cqrs::CqrsError;
use gerax_db::DbError;

use crate::domain::DomainError;

/// Categoria do erro da aplicação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationErrorKind {
    /// Entidade não encontrada.
    NotFound,
    /// Violação de regra de negócio.
    BusinessRule,
    /// Falha de infraestrutura (banco, rede, serialização).
    Infrastructure,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ApplicationError {
    kind: ApplicationErrorKind,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ApplicationError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: ApplicationErrorKind::NotFound,
            message: message.into(),
            source: None,
        }
    }

    pub fn business_rule(message: impl Into<String>) -> Self {
        Self {
            kind: ApplicationErrorKind::BusinessRule,
            message: message.into(),
            source: None,
        }
    }

    pub fn infrastructure(message: impl Into<String>) -> Self {
        Self {
            kind: ApplicationErrorKind::Infrastructure,
            message: message.into(),
            source: None,
        }
    }

    pub fn infrastructure_err<E>(message: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind: ApplicationErrorKind::Infrastructure,
            message: message.into(),
            source: Some(Box::new(err)),
        }
    }

    /// Converte em `CqrsError`, preservando a categoria através de uma
    /// tag prefixada na mensagem (`[not_found]`, `[business_rule]`,
    /// `[infrastructure]`). O adaptador HTTP usa essa tag para escolher
    /// o status code.
    pub fn into_cqrs(self) -> CqrsError {
        let tag = match self.kind {
            ApplicationErrorKind::NotFound => "not_found",
            ApplicationErrorKind::BusinessRule => "business_rule",
            ApplicationErrorKind::Infrastructure => "infrastructure",
        };
        CqrsError::Execution(format!("[{tag}] {}", self.message))
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApplicationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|s| s.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl From<DomainError> for ApplicationError {
    fn from(err: DomainError) -> Self {
        ApplicationError::business_rule(err.to_string())
    }
}

/// Mapeia os erros de persistência do `gerax-db` para a linguagem da
/// aplicação, preservando a categoria `NotFound`.
impl From<DbError> for ApplicationError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFoundError(msg) => {
                ApplicationError::not_found(format!("entidade não encontrada: {msg}"))
            }
            DbError::ConfigurationError(msg) => {
                ApplicationError::infrastructure(format!("configuração: {msg}"))
            }
            DbError::ConnectionError(msg) => {
                ApplicationError::infrastructure(format!("conexão: {msg}"))
            }
            DbError::SerializationError(msg) => {
                ApplicationError::infrastructure(format!("serialização: {msg}"))
            }
        }
    }
}
