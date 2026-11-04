//! Porta de origem de eventos para o Event Relay.
//!
//! A `gerax-eda` **não** depende de KurrentDB, NATS ou qualquer outro
//! transporte. Adapters implementam [`EventSource`] e traduzem os seus
//! eventos para [`IntegrationEvent`].

use async_trait::async_trait;
use thiserror::Error;

use super::envelope::IntegrationEvent;

/// Ação de confirmação devolvida pelo relay à origem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryAction {
    /// Processamento concluído com sucesso.
    Ack,
    /// Falha transitória: a origem deve reentregar o evento.
    Retry,
    /// Falha permanente ou retries esgotados: enviar para park.
    Park,
    /// Ignorar o evento sem reprocessar e sem park.
    Skip,
}

/// Evento entregue pela origem, com o número de tentativas anterior.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceEvent {
    /// Envelope de integração.
    pub event: IntegrationEvent,
    /// Quantidade de entregas anteriores (0 = primeira entrega).
    pub retry_count: u32,
}

impl SourceEvent {
    /// Cria um evento na primeira entrega.
    pub fn new(event: IntegrationEvent) -> Self {
        Self {
            event,
            retry_count: 0,
        }
    }

    /// Define o número de entregas anteriores (builder).
    pub fn with_retry_count(mut self, retry_count: u32) -> Self {
        self.retry_count = retry_count;
        self
    }

    /// `true` quando o evento é uma reentrega.
    pub fn is_redelivery(&self) -> bool {
        self.retry_count > 0
    }
}

/// Erro produzido por uma origem de eventos.
#[derive(Debug, Error)]
pub enum SourceError {
    /// A origem terminou normalmente (sem mais eventos).
    #[error("event source closed")]
    Closed,

    /// Falha transitória de transporte/conexão.
    #[error("retryable event source error: {0}")]
    Retryable(String),

    /// Falha definitiva da origem.
    #[error("fatal event source error: {0}")]
    Fatal(String),
}

impl SourceError {
    /// `true` quando a operação pode ser repetida.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }

    /// `true` quando a falha é definitiva.
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }

    /// `true` quando a origem terminou normalmente.
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed)
    }
}

/// Porta de consumo de eventos para o Event Relay.
///
/// O relay busca um evento, processa e então confirma com
/// [`DeliveryAction`]. O ACK é responsabilidade da origem — no
/// KurrentDB, ele só deve ocorrer após a publicação confirmada.
#[async_trait]
pub trait EventSource: Send {
    /// Aguarda o próximo evento.
    ///
    /// Devolve [`SourceError::Closed`] quando não há mais eventos e a
    /// origem terminou normalmente.
    async fn next_event(&mut self) -> Result<SourceEvent, SourceError>;

    /// Confirma o processamento do evento com a ação indicada.
    async fn acknowledge(
        &mut self,
        event: &IntegrationEvent,
        action: DeliveryAction,
    ) -> Result<(), SourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_event_marca_reentrega() {
        let event = IntegrationEvent::new(
            gerax_es::EventId::new(),
            "AlunoMatriculado",
            "aluno-1",
            "Aluno",
            0,
            chrono::Utc::now(),
            serde_json::json!({}),
        );

        let first = SourceEvent::new(event.clone());
        assert!(!first.is_redelivery());

        let retry = SourceEvent::new(event).with_retry_count(2);
        assert!(retry.is_redelivery());
        assert_eq!(retry.retry_count, 2);
    }

    #[test]
    fn classificacao_dos_erros_e_estavel() {
        assert!(SourceError::Closed.is_closed());
        assert!(SourceError::Retryable("offline".into()).is_retryable());
        assert!(SourceError::Fatal("invalid".into()).is_fatal());
        assert!(!SourceError::Fatal("invalid".into()).is_retryable());
    }
}
