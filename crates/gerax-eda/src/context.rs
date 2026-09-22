//! Contexto de uma tentativa de entrega.

use chrono::{DateTime, Utc};
use gerax_es::EventId;

use crate::delivery::EventDelivery;
use crate::envelope::EventEnvelope;
use crate::event::Event;
use crate::metadata::{CausationId, CorrelationId};

/// Leitura imutável de uma tentativa de entrega, entregue ao
/// [`EventMiddleware`](crate::EventMiddleware).
///
/// Reúne identidade, rastreio e tentativa corrente sem expor o
/// payload do evento — suficiente para logging, tracing, métricas e
/// auditoria de correlation/causation.
#[derive(Debug, Clone, PartialEq)]
pub struct EventContext {
    /// Tentativa corrente (event_id + número da tentativa).
    pub delivery: EventDelivery,
    /// Nome lógico do evento.
    pub event_type: String,
    /// Correlação do fluxo, se conhecida.
    pub correlation_id: Option<CorrelationId>,
    /// Evento que originou este, se conhecido.
    pub causation_id: Option<CausationId>,
    /// Instante de ocorrência do evento.
    pub occurred_at: DateTime<Utc>,
    /// Nome Rust do handler que está prestes a ser chamado.
    pub handler: &'static str,
}

impl EventContext {
    /// Constrói o contexto de uma tentativa a partir do envelope.
    pub fn new<E>(envelope: &EventEnvelope<E>, handler: &'static str, attempt: u32) -> Self
    where
        E: Event,
    {
        Self {
            delivery: EventDelivery {
                event_id: envelope.id.clone(),
                attempt,
            },
            event_type: envelope.metadata.event_type.clone(),
            correlation_id: envelope.metadata.correlation_id.clone(),
            causation_id: envelope.metadata.causation_id.clone(),
            occurred_at: envelope.metadata.occurred_at,
            handler,
        }
    }

    /// Identificador do evento em esta tentativa.
    pub fn event_id(&self) -> &EventId {
        &self.delivery.event_id
    }

    /// Número da tentativa corrente.
    pub const fn attempt(&self) -> u32 {
        self.delivery.attempt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct AlunoMatriculado;

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[test]
    fn context_reflete_o_envelope_e_a_tentativa() {
        let envelope = EventEnvelope::new(AlunoMatriculado)
            .with_correlation_id("fluxo-1")
            .with_causation_id("evt-0");

        let context = EventContext::new(&envelope, "AtualizarDiario", 2);

        assert_eq!(context.event_id(), &envelope.id);
        assert_eq!(context.event_type, "aluno.matriculado");
        assert_eq!(context.attempt(), 2);
        assert_eq!(context.handler, "AtualizarDiario");
        assert_eq!(
            context.correlation_id.as_ref().map(|c| c.as_str()),
            Some("fluxo-1")
        );
        assert_eq!(
            context.causation_id.as_ref().map(|c| c.as_str()),
            Some("evt-0")
        );
        assert_eq!(context.occurred_at, envelope.metadata.occurred_at);
    }

    #[test]
    fn context_sem_rastreio_permite_none() {
        let envelope = EventEnvelope::new(AlunoMatriculado);
        let context = EventContext::new(&envelope, "H", 1);

        assert!(context.correlation_id.is_none());
        assert!(context.causation_id.is_none());
        assert_eq!(context.delivery.attempt, 1);
    }
}
