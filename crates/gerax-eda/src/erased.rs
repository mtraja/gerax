//! Type erasure explícita para armazenamento heterogêneo de handlers.
//!
//! O bus precisa entregar o mesmo evento a handlers de diferentes
//! tipos. O fluxo é explícito — nada de `Any` espalhado pela API:
//!
//! ```text
//! EventEnvelope<E>
//!      │  ErasedEvent::new
//!      ▼
//! ErasedEvent          (id + type_id + event_type + envelope)
//!      │  handlers_for(TypeId)
//!      ▼
//! ErasedEventHandler   (downcast de volta para EventEnvelope<E>)
//!      │
//!      ▼
//! EventHandler<E>
//! ```
//!
//! Este módulo é detalhe interno da infraestrutura (mesma abordagem
//! do `erased.rs` de `gerax-cqrs`), com um adicional: a caixa também
//! carrega metadados diagnósticos para que o registry e o bus
//! roteiem sem reabrir o payload.

use std::any::{Any, TypeId};

use async_trait::async_trait;

use crate::envelope::EventEnvelope;
use crate::error::EventHandlerError;
use crate::event::Event;

/// Evento apagado (*type-erased*): unidade de roteamento do registry.
///
/// Congela a informação necessária ao roteamento (`type_id`,
/// `event_id`, `event_type`) e guarda o envelope concreto em uma caixa
/// tipada apenas internamente.
pub struct ErasedEvent {
    type_id: TypeId,
    event_id: gerax_es::EventId,
    event_type: String,
    envelope: Box<dyn Any + Send + Sync>,
}

impl ErasedEvent {
    /// Apaga um envelope concreto preservando os dados de roteamento.
    pub fn new<E>(envelope: &EventEnvelope<E>) -> Self
    where
        E: Event + Clone,
    {
        Self {
            type_id: TypeId::of::<E>(),
            event_id: envelope.id.clone(),
            event_type: envelope.metadata.event_type.clone(),
            envelope: Box::new(envelope.clone()),
        }
    }

    /// `TypeId` do tipo concreto do evento (`E`).
    pub const fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Identificador do evento (mesmo do envelope original).
    pub fn event_id(&self) -> &gerax_es::EventId {
        &self.event_id
    }

    /// Nome lógico do evento.
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Recupera o envelope concreto, se o tipo corresponder.
    pub fn downcast<E>(&self) -> Option<&EventEnvelope<E>>
    where
        E: Event,
    {
        self.envelope.downcast_ref::<EventEnvelope<E>>()
    }

    /// Recupera o envelope concreto **clonado**, se o tipo corresponder.
    pub fn to_envelope<E>(&self) -> Option<EventEnvelope<E>>
    where
        E: Event + Clone,
    {
        self.downcast::<E>().cloned()
    }
}

/// Handler apagado: interface de despacho do registry.
#[async_trait]
pub trait ErasedEventHandler: Send + Sync {
    /// Nome Rust do handler concreto (diagnóstico/observabilidade).
    fn name(&self) -> &'static str;

    /// Executa o handler após `downcast` do envelope.
    async fn handle(&self, event: &ErasedEvent) -> Result<(), EventHandlerError>;
}

/// Adapta um [`crate::EventHandler<E>`] ao [`ErasedEventHandler`].
pub struct EventHandlerAdapter<H, E> {
    pub(crate) handler: H,
    pub(crate) _marker: std::marker::PhantomData<fn() -> E>,
}

impl<H, E> EventHandlerAdapter<H, E> {
    pub(crate) fn new(handler: H) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<H, E> ErasedEventHandler for EventHandlerAdapter<H, E>
where
    H: crate::EventHandler<E>,
    E: Event + Clone,
{
    fn name(&self) -> &'static str {
        std::any::type_name::<H>()
    }

    async fn handle(&self, event: &ErasedEvent) -> Result<(), EventHandlerError> {
        let envelope = event.to_envelope::<E>().ok_or_else(|| {
            EventHandlerError::ProcessingFailed(format!(
                "event type mismatch: registry entry expects `{}` but received `{}`",
                std::any::type_name::<E>(),
                event.event_type(),
            ))
        })?;

        self.handler.handle(envelope).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::EventHandlerError;

    #[derive(Debug, Clone, PartialEq)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct OutroEvento;

    impl Event for OutroEvento {
        fn event_type(&self) -> &'static str {
            "outro.evento"
        }
    }

    struct Recebedor;

    #[async_trait]
    impl crate::EventHandler<AlunoMatriculado> for Recebedor {
        async fn handle(
            &self,
            event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            assert_eq!(event.event.aluno_id, "a-1");
            Ok(())
        }
    }

    #[tokio::test]
    async fn erased_preserva_roteamento_e_payload() {
        let envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        });
        let erased = ErasedEvent::new(&envelope);

        assert_eq!(erased.type_id(), TypeId::of::<AlunoMatriculado>());
        assert_eq!(erased.event_type(), "aluno.matriculado");
        assert_eq!(erased.event_id(), &envelope.id);
        assert_eq!(erased.downcast::<AlunoMatriculado>(), Some(&envelope));
        assert!(erased.downcast::<OutroEvento>().is_none());
    }

    #[tokio::test]
    async fn adapter_despacha_para_handler_tipado() {
        let envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        });
        let erased = ErasedEvent::new(&envelope);
        let adapter = EventHandlerAdapter::<_, AlunoMatriculado>::new(Recebedor);

        assert_eq!(adapter.name(), std::any::type_name::<Recebedor>());
        adapter.handle(&erased).await.unwrap();
    }

    #[tokio::test]
    async fn adapter_rejeita_tipo_incompativel() {
        let envelope = EventEnvelope::new(OutroEvento);
        let erased = ErasedEvent::new(&envelope);
        let adapter = EventHandlerAdapter::<_, AlunoMatriculado>::new(Recebedor);

        let err = adapter.handle(&erased).await.unwrap_err();
        assert!(matches!(err, EventHandlerError::ProcessingFailed(_)));
    }
}
