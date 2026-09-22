//! Consumidores de eventos.

use std::future::Future;

use async_trait::async_trait;

use crate::envelope::EventEnvelope;
use crate::error::EventHandlerError;
use crate::event::Event;

/// Consome eventos de um tipo `E`.
///
/// O handler conhece o evento, mas **não** o mecanismo de transporte:
/// ele recebe um [`EventEnvelope`] e devolve `Ok` ou um
/// [`EventHandlerError`] classificado.
///
/// ```text
/// EventBus
///    │
///    ├── EventHandler<E>   (projeção)
///    ├── EventHandler<E>   (notificação)
///    └── EventHandler<E>   (integração)
/// ```
///
/// Vários handlers podem consumir o mesmo evento; o registro é
/// feito via [`EventSubscriber::register`](crate::EventSubscriber::register)
/// ou [`InMemoryEventBus::register`](crate::InMemoryEventBus::register).
#[async_trait]
pub trait EventHandler<E>: Send + Sync
where
    E: Event,
{
    /// Processa uma entrega do evento.
    async fn handle(&self, event: EventEnvelope<E>) -> Result<(), EventHandlerError>;
}

/// Permite usar closures como [`EventHandler`].
///
/// ```ignore
/// bus.register::<AlunoMatriculado>(|envelope| async move {
///     println!("{}", envelope.event.aluno_id);
///     Ok(())
/// });
/// ```
#[async_trait]
impl<E, F, Fut> EventHandler<E> for F
where
    E: Event,
    F: Fn(EventEnvelope<E>) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), EventHandlerError>> + Send,
{
    async fn handle(&self, event: EventEnvelope<E>) -> Result<(), EventHandlerError> {
        (self)(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone)]
    struct AlunoMatriculado;

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    struct Contador {
        total: AtomicUsize,
    }

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for Contador {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            self.total.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn handler_struct_recebe_envelope() {
        let contador = Contador {
            total: AtomicUsize::new(0),
        };

        contador
            .handle(EventEnvelope::new(AlunoMatriculado))
            .await
            .unwrap();

        assert_eq!(contador.total.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn closure_eh_handler() {
        let total = Arc::new(AtomicUsize::new(0));
        let capturado = total.clone();

        let handler = move |_envelope: EventEnvelope<AlunoMatriculado>| {
            let capturado = capturado.clone();
            async move {
                capturado.fetch_add(1, Ordering::SeqCst);
                Ok::<(), EventHandlerError>(())
            }
        };

        handler
            .handle(EventEnvelope::new(AlunoMatriculado))
            .await
            .unwrap();
        assert_eq!(total.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn handler_pode_propagar_erro() {
        let handler = |_envelope: EventEnvelope<AlunoMatriculado>| async {
            Err::<(), _>(EventHandlerError::Retryable("timeout".into()))
        };

        let err = handler
            .handle(EventEnvelope::new(AlunoMatriculado))
            .await
            .unwrap_err();
        assert!(err.is_retryable());
    }
}
