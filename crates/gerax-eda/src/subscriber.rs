//! Assinatura de eventos (lado consumidor).

use std::sync::Arc;

use crate::event::Event;
use crate::handler::EventHandler;

/// Lado consumidor do bus: registro de handlers (assinaturas).
///
/// ```text
/// EventBus (publica)
///    │
///    ▼
/// EventSubscriber (assina)
///    │
///    ├── EventHandler<E>
///    ├── EventHandler<E>
///    └── EventHandler<E>
/// ```
///
/// Separa a capacidade de **publicar** ([`crate::EventBus`]) da
/// capacidade de **consumir** ([`EventSubscriber`]), permitindo que
/// componentes exponham apenas o que precisam.
///
/// Registro aceita **N handlers por evento**; não deduplica (ver
/// [`crate::InMemoryEventBus`] para a semântica de entrega).
pub trait EventSubscriber: Send + Sync {
    /// Assina o tipo de evento `E` com um handler.
    ///
    /// `E: Clone` porque a entrega em fan-out clona o envelope por
    /// handler.
    fn register<E, H>(&self, handler: H)
    where
        E: Event + Clone,
        H: EventHandler<E> + 'static;

    /// Quantidade de handlers assinados para `E`.
    fn handler_count<E: Event>(&self) -> usize;
}

/// Permite assinar através de `Arc<S>`.
impl<S> EventSubscriber for Arc<S>
where
    S: EventSubscriber + ?Sized,
{
    fn register<E, H>(&self, handler: H)
    where
        E: Event + Clone,
        H: EventHandler<E> + 'static,
    {
        (**self).register::<E, H>(handler);
    }

    fn handler_count<E: Event>(&self) -> usize {
        (**self).handler_count::<E>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::EventEnvelope;
    use crate::error::EventHandlerError;
    use crate::in_memory::InMemoryEventBus;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone)]
    struct AlunoMatriculado;

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    struct Contador(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl EventHandler<AlunoMatriculado> for Contador {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn subscriber_registra_e_conta() {
        let total = Arc::new(AtomicUsize::new(0));
        let bus = InMemoryEventBus::new();

        assert_eq!(bus.handler_count::<AlunoMatriculado>(), 0);
        bus.register::<AlunoMatriculado, _>(Contador(total.clone()));
        assert_eq!(bus.handler_count::<AlunoMatriculado>(), 1);
    }

    #[tokio::test]
    async fn subscriber_funciona_atraves_de_arc() {
        let total = Arc::new(AtomicUsize::new(0));
        let bus = Arc::new(InMemoryEventBus::new());

        EventSubscriber::register::<AlunoMatriculado, _>(bus.as_ref(), Contador(total.clone()));
        assert_eq!(
            EventSubscriber::handler_count::<AlunoMatriculado>(bus.as_ref()),
            1
        );
    }
}
