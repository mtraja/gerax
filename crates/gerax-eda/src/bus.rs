//! Porta principal de publicação de eventos.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::envelope::EventEnvelope;
use crate::error::EventBusError;
use crate::event::Event;

/// Porta de distribuição de eventos: o coração do EDA.
///
/// ```text
/// Event → Envelope → EventBus → Subscribers → EventHandlers
/// ```
///
/// Separa responsabilidades:
///
/// ```text
/// EventStore   (gerax-es)   persiste o que aconteceu
/// EventPublisher             transição persistidos → distribuídos
/// EventBus     (gerax-eda)   distribui o que aconteceu
/// ```
///
/// O domínio **não** conhece o `EventBus`; apenas camadas de
/// infraestrutura (projections, publishers) o utilizam.
///
/// # Type erasure
///
/// O método [`EventBus::publish`] é genérico no tipo do evento, mas
/// por baixo os envelopes passam por [`crate::erased::ErasedEvent`]
/// para que o registry armazene handlers heterogêneos — o usuário da
/// crate não lida com `Any` diretamente.
///
/// # Exemplo
///
/// ```rust,no_run
/// use gerax_eda::{Event, EventBus, EventEnvelope, EventHandler,
///                 EventHandlerError, InMemoryEventBus};
/// use serde::Serialize;
///
/// #[derive(Debug, Clone, Serialize)]
/// struct AlunoMatriculado { aluno_id: String }
///
/// impl Event for AlunoMatriculado {
///     fn event_type(&self) -> &'static str { "aluno.matriculado" }
/// }
///
/// struct LogHandler;
///
/// #[async_trait::async_trait]
/// impl EventHandler<AlunoMatriculado> for LogHandler {
///     async fn handle(
///         &self,
///         envelope: EventEnvelope<AlunoMatriculado>,
///     ) -> Result<(), EventHandlerError> {
///         println!("matriculado: {}", envelope.event.aluno_id);
///         Ok(())
///     }
/// }
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let bus = InMemoryEventBus::new();
/// bus.register::<AlunoMatriculado, _>(LogHandler);
/// bus.publish(EventEnvelope::new(AlunoMatriculado {
///     aluno_id: "a-1".into(),
/// })).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publica um envelope para distribuição.
    ///
    /// Contrato da porta — vale para **todas** as implementações
    /// (dispatch local e adapters de broker: RabbitMQ, NATS, Kafka…):
    ///
    /// 1. valida `metadata.event_type` contra [`Event::event_type`] →
    ///    [`EventBusError::InvalidEvent`];
    /// 2. entrega/publica o evento com semântica *at-least-once* e
    ///    **nunca falha silenciosamente** — falha de transporte vira
    ///    [`EventBusError::PublishFailed`];
    /// 3. a semântica de **entrega** é específica da implementação:
    ///    [`crate::InMemoryEventBus`] faz dispatch local (fan-out a
    ///    handlers registrados, [`EventBusError::HandlerNotFound`],
    ///    retry, middleware); adapters de broker publicam no
    ///    transporte — não há handlers locais, e retry/DLQ pertencem
    ///    ao consumer do adapter (ver [`crate::adapters`]).
    ///
    /// O bound `Serialize` existe porque toda implementação precisa
    /// representar o payload fora do tipo Rust — brokers o levam ao
    /// wire; [`crate::InMemoryEventBus`] aceita o bound sem usá-lo.
    ///
    /// O bound `Clone` suporta o dispatch local: o evento é entregue
    /// a N handlers, cada um com o seu [`EventEnvelope`] próprio.
    async fn publish<E>(&self, event: EventEnvelope<E>) -> Result<(), EventBusError>
    where
        E: Event + Clone + Serialize;
}

/// Permite compartilhar qualquer bus por referência
/// (`Arc<dyn-compatible bus>`).
#[async_trait]
impl<B> EventBus for Arc<B>
where
    B: EventBus + ?Sized,
{
    async fn publish<E>(&self, event: EventEnvelope<E>) -> Result<(), EventBusError>
    where
        E: Event + Clone + Serialize,
    {
        (**self).publish(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::EventHandler;
    use crate::subscriber::EventSubscriber;
    use std::sync::Arc;

    #[derive(Debug, Clone, Serialize)]
    struct AlunoMatriculado;

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    /// Bus mínimo de teste para validar o contrato da porta.
    struct NullBus;

    #[async_trait]
    impl EventSubscriber for NullBus {
        fn register<E, H>(&self, _handler: H)
        where
            E: Event + Clone,
            H: EventHandler<E> + 'static,
        {
        }

        fn handler_count<E: Event>(&self) -> usize {
            0
        }
    }

    #[async_trait]
    impl EventBus for NullBus {
        async fn publish<E>(&self, _event: EventEnvelope<E>) -> Result<(), EventBusError>
        where
            E: Event + Clone + Serialize,
        {
            Err(EventBusError::HandlerNotFound {
                event_type: "null".into(),
            })
        }
    }

    #[tokio::test]
    async fn porta_de_event_bus_e_utilizavel_genericamente() {
        async fn publicar<B: EventBus>(bus: &B) -> Result<(), EventBusError> {
            bus.publish(EventEnvelope::new(AlunoMatriculado)).await
        }

        let err = publicar(&NullBus).await.unwrap_err();
        assert!(matches!(err, EventBusError::HandlerNotFound { .. }));
    }

    #[tokio::test]
    async fn arc_compartilha_o_bus() {
        let bus: Arc<NullBus> = Arc::new(NullBus);

        // publish via Arc<B: EventBus> (blanket impl de Arc)
        let resultado = bus.publish(EventEnvelope::new(AlunoMatriculado)).await;

        assert!(matches!(
            resultado,
            Err(EventBusError::HandlerNotFound { .. })
        ));
    }
}
