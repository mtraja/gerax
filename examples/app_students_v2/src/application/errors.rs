use std::fmt;

use crate::domain::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationErrorKind {
    NotFound,
    BusinessRule,
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

    #[allow(dead_code)]
    pub fn kind(&self) -> ApplicationErrorKind {
        self.kind
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
        ApplicationError::infrastructure(err.to_string())
    }
}
