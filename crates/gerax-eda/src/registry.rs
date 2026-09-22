//! Registro de handlers por tipo de evento.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use crate::erased::{ErasedEvent, ErasedEventHandler, EventHandlerAdapter};
use crate::event::Event;
use crate::handler::EventHandler;

/// Registro heterogêneo de [`EventHandler`]s, com suporte a **N
/// handlers para o mesmo evento**.
///
/// ```text
/// AlunoMatriculado
///        │
///        ├── AtualizarDiario
///        ├── AtualizarFinanceiro
///        └── EnviarNotificacao
/// ```
///
/// ## Chave de roteamento
///
/// O registro é indexado por [`TypeId`] de `E` — o mesmo mecanismo
/// usado pelo `HandlerRegistry` de `gerax-cqrs`. O nome lógico
/// ([`Event::event_type`]) permanece no envelope como **contrato
/// externo** estável, independentemente do nome Rust completo.
///
/// ## Multi-handler
///
/// [`EventHandlerRegistry::register`] acrescenta ao vetor do tipo;
/// registrar o mesmo handler duas vezes significa processar o evento
/// duas vezes (o registry **não** deduplica — ver a semântica de
/// entrega em [`crate::InMemoryEventBus`]).
pub struct EventHandlerRegistry {
    handlers: HashMap<TypeId, Vec<Arc<dyn ErasedEventHandler>>>,
}

impl EventHandlerRegistry {
    /// Cria um registry vazio.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registra um handler para o tipo de evento `E`.
    ///
    /// Suporta múltiplos handlers por evento; não há limite e não há
    /// erro em duplicar (deduplicação é responsabilidade do chamador).
    pub fn register<E, H>(&mut self, handler: H)
    where
        E: Event + Clone,
        H: EventHandler<E> + 'static,
    {
        self.handlers
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Arc::new(EventHandlerAdapter::<H, E>::new(handler)));
    }

    /// Quantidade de handlers registrados para `E`.
    pub fn handler_count<E: Event>(&self) -> usize {
        self.handlers
            .get(&TypeId::of::<E>())
            .map(Vec::len)
            .unwrap_or(0)
    }

    /// `true` quando existe ao menos um handler para `E`.
    pub fn has_handlers<E: Event>(&self) -> bool {
        self.handler_count::<E>() > 0
    }

    /// Handlers de `E` em ordem de registro (clones de referência).
    ///
    /// Usado pelo bus para despachar sem segurar lock durante `await`.
    pub fn handlers_for<E: Event>(&self) -> Vec<Arc<dyn ErasedEventHandler>> {
        self.handlers
            .get(&TypeId::of::<E>())
            .cloned()
            .unwrap_or_default()
    }

    /// Despacha um envelope já apagado para os handlers de `E`.
    ///
    /// Devolve as falhas (uma por handler que falhou); lista vazia
    /// significa que todos processaram com sucesso. Os handlers são
    /// executados **em ordem de registro**, de forma isolada: a falha
    /// de um não impede os demais.
    pub async fn dispatch(&self, event: &ErasedEvent) -> Vec<crate::error::HandlerFailure> {
        let Some(handlers) = self.handlers.get(&event.type_id()) else {
            return Vec::new();
        };

        let mut failures = Vec::new();
        for handler in handlers {
            if let Err(error) = handler.handle(event).await {
                failures.push(crate::error::HandlerFailure {
                    handler: handler.name(),
                    error,
                });
            }
        }
        failures
    }
}

impl Default for EventHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::EventEnvelope;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::error::EventHandlerError;

    #[derive(Debug, Clone)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[derive(Debug, Clone)]
    struct TurmaCriada;

    impl Event for TurmaCriada {
        fn event_type(&self) -> &'static str {
            "turma.criada"
        }
    }

    #[derive(Clone)]
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

    struct Falhador;

    #[async_trait::async_trait]
    impl EventHandler<AlunoMatriculado> for Falhador {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            Err(EventHandlerError::ProcessingFailed("boom".into()))
        }
    }

    #[tokio::test]
    async fn register_depois_has_handlers_e_count() {
        let mut registry = EventHandlerRegistry::new();
        assert!(!registry.has_handlers::<AlunoMatriculado>());
        assert_eq!(registry.handler_count::<AlunoMatriculado>(), 0);

        registry.register::<AlunoMatriculado, _>(Contador(Arc::new(AtomicUsize::new(0))));

        assert!(registry.has_handlers::<AlunoMatriculado>());
        assert_eq!(registry.handler_count::<AlunoMatriculado>(), 1);
        assert!(!registry.has_handlers::<TurmaCriada>());
    }

    #[tokio::test]
    async fn multiplos_handlers_para_o_mesmo_evento() {
        let total_a = Arc::new(AtomicUsize::new(0));
        let total_b = Arc::new(AtomicUsize::new(0));

        let mut registry = EventHandlerRegistry::new();
        registry.register::<AlunoMatriculado, _>(Contador(total_a.clone()));
        registry.register::<AlunoMatriculado, _>(Contador(total_b.clone()));
        registry.register::<AlunoMatriculado, _>(Contador(total_a.clone()));

        assert_eq!(registry.handler_count::<AlunoMatriculado>(), 3);

        let envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        });
        assert_eq!(envelope.event.aluno_id, "a-1");
        let erased = ErasedEvent::new(&envelope);
        let failures = registry.dispatch(&erased).await;

        assert!(failures.is_empty());
        assert_eq!(total_a.load(Ordering::SeqCst), 2);
        assert_eq!(total_b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn despacho_isola_falha_de_um_handler() {
        let total = Arc::new(AtomicUsize::new(0));

        let mut registry = EventHandlerRegistry::new();
        registry.register::<AlunoMatriculado, _>(Falhador);
        registry.register::<AlunoMatriculado, _>(Contador(total.clone()));

        let envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        });
        let erased = ErasedEvent::new(&envelope);
        let failures = registry.dispatch(&erased).await;

        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].handler, std::any::type_name::<Falhador>());
        // handler seguinte ainda executou
        assert_eq!(total.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn dispatch_sem_handlers_retorna_vazio() {
        let registry = EventHandlerRegistry::new();
        let envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        });
        let erased = ErasedEvent::new(&envelope);

        assert!(registry.dispatch(&erased).await.is_empty());
    }

    #[tokio::test]
    async fn registry_padrao_e_vazio() {
        let registry = EventHandlerRegistry::default();
        assert!(!registry.has_handlers::<AlunoMatriculado>());
    }
}
