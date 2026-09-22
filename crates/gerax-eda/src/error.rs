//! Erros de distribuição de eventos (EDA).

use thiserror::Error;

/// Erro produzido por um [`EventHandler`](crate::EventHandler) durante o
/// processamento de um evento.
///
/// A classificação do erro alimenta a política de retry do
/// [`RetryPolicy`](crate::RetryPolicy):
///
/// - [`EventHandlerError::Retryable`] — falha transitória; o bus tenta
///   novamente até esgotar `max_attempts`;
/// - [`EventHandlerError::NonRetryable`] e
///   [`EventHandlerError::ProcessingFailed`] — falha definitiva; nenhum
///   retry é executado.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EventHandlerError {
    /// Falha de processamento genérica (não retentável por padrão).
    #[error("processing failed: {0}")]
    ProcessingFailed(String),

    /// Falha transitória passível de retry.
    #[error("retryable failure: {0}")]
    Retryable(String),

    /// Falha definitiva que não deve ser retentada.
    #[error("non-retryable failure: {0}")]
    NonRetryable(String),
}

impl EventHandlerError {
    /// Retorna `true` quando a falha pode ser retentada.
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }
}

/// Falha individual de um handler dentro de uma publicação.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{handler}: {error}")]
pub struct HandlerFailure {
    /// Nome do handler que falhou (nome Rust do tipo).
    pub handler: &'static str,
    /// Erro devolvido pelo handler.
    pub error: EventHandlerError,
}

/// Erros produzidos pelo [`EventBus`](crate::EventBus).
#[derive(Debug, Error)]
pub enum EventBusError {
    /// Nenhum handler está registrado para o tipo do evento publicado.
    ///
    /// Política documentada: publicar um evento sem consumidores é um
    /// erro de infraestrutura (registro ausente), não um no-op silencioso.
    #[error("no handlers registered for event `{event_type}`")]
    HandlerNotFound {
        /// Tipo lógico do evento sem handlers.
        event_type: String,
    },

    /// Ao menos um handler falhou após esgotar a política de retry.
    ///
    /// Todos os demais handlers do mesmo evento **executaram** antes
    /// deste erro (falhas são isoladas por handler).
    #[error(
        "handler failed for event `{event_type}`: {}",
        .failures.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
    )]
    HandlerFailed {
        /// Tipo lógico do evento publicado.
        event_type: String,
        /// Falhas individuais, uma por handler que falhou.
        failures: Vec<HandlerFailure>,
    },

    /// O envelope é inconsistente com o evento que carrega.
    ///
    /// Ocorre quando `metadata.event_type` não corresponde a
    /// [`Event::event_type()`](crate::Event::event_type).
    #[error("invalid event: {0}")]
    InvalidEvent(String),

    /// Falha genérica de publicação (extensível para adapters).
    #[error("publish failed: {0}")]
    PublishFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apenas_retryable_e_retentavel() {
        assert!(EventHandlerError::Retryable("flaky".into()).is_retryable());
        assert!(!EventHandlerError::NonRetryable("bad".into()).is_retryable());
        assert!(!EventHandlerError::ProcessingFailed("boom".into()).is_retryable());
    }

    #[test]
    fn handler_failure_exibe_handler_e_erro() {
        let failure = HandlerFailure {
            handler: "AtualizarDiario",
            error: EventHandlerError::ProcessingFailed("db down".into()),
        };

        assert_eq!(
            failure.to_string(),
            "AtualizarDiario: processing failed: db down"
        );
    }

    #[test]
    fn handler_failed_resume_todas_as_falhas() {
        let err = EventBusError::HandlerFailed {
            event_type: "AlunoMatriculado".into(),
            failures: vec![
                HandlerFailure {
                    handler: "H1",
                    error: EventHandlerError::Retryable("timeout".into()),
                },
                HandlerFailure {
                    handler: "H2",
                    error: EventHandlerError::NonRetryable("invalid".into()),
                },
            ],
        };

        let message = err.to_string();
        assert!(message.contains("AlunoMatriculado"));
        assert!(message.contains("H1: retryable failure: timeout"));
        assert!(message.contains("H2: non-retryable failure: invalid"));
    }

    #[test]
    fn handler_not_found_exibe_tipo_do_evento() {
        let err = EventBusError::HandlerNotFound {
            event_type: "AlunoMatriculado".into(),
        };
        assert_eq!(
            err.to_string(),
            "no handlers registered for event `AlunoMatriculado`"
        );
    }
}
