//! Entrega e política de retry.

use gerax_es::EventId;

use crate::error::EventHandlerError;

/// Identificação de uma tentativa de entrega de evento.
///
/// Base para recursos futuros de infraestrutura — retry avançado,
/// dead-letter, backoff, observabilidade e idempotência — sem
/// implementar um sistema distribuído na primeira versão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDelivery {
    /// Identificador do evento sendo entregue.
    pub event_id: EventId,
    /// Número da tentativa (1 = primeira entrega).
    pub attempt: u32,
}

impl EventDelivery {
    /// Cria uma entrega na primeira tentativa.
    pub fn first(event_id: EventId) -> Self {
        Self {
            event_id,
            attempt: 1,
        }
    }

    /// Avança para a próxima tentativa.
    pub fn next_attempt(mut self) -> Self {
        self.attempt += 1;
        self
    }
}

/// Política de retry **simples** (sem backoff, sem broker).
///
/// ```text
/// attempt 1 → falhou Retryable → attempt 2 → falhou Retryable → ...
///                               até max_attempts
/// ```
///
/// Apenas [`EventHandlerError::Retryable`] é retentado; erros
/// [`EventHandlerError::NonRetryable`] e
/// [`EventHandlerError::ProcessingFailed`] falham imediatamente.
///
/// `max_attempts = 1` (padrão) desativa o retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Número máximo de tentativas por handler (mínimo: 1).
    pub max_attempts: u32,
}

impl RetryPolicy {
    /// Cria uma política com o número máximo de tentativas.
    ///
    /// Valores `0` são elevados a `1` (sempre há ao menos uma
    /// tentativa).
    pub const fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts: if max_attempts == 0 { 1 } else { max_attempts },
        }
    }

    /// Número efetivo de tentativas (nunca menor que 1).
    pub const fn effective_max_attempts(self) -> u32 {
        if self.max_attempts == 0 {
            1
        } else {
            self.max_attempts
        }
    }

    /// Decide se a tentativa `attempt` deve ser repetida após `error`.
    pub fn should_retry(self, attempt: u32, error: &EventHandlerError) -> bool {
        error.is_retryable() && attempt < self.effective_max_attempts()
    }
}

impl Default for RetryPolicy {
    /// Sem retry: uma única tentativa por handler.
    fn default() -> Self {
        Self { max_attempts: 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_first_e_next_attempt() {
        let delivery = EventDelivery::first(EventId::new());
        assert_eq!(delivery.attempt, 1);

        let segunda = delivery.clone().next_attempt();
        assert_eq!(segunda.attempt, 2);
        assert_eq!(segunda.event_id, delivery.event_id);
    }

    #[test]
    fn padrao_e_uma_tentativa_sem_retry() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 1);
        assert!(!policy.should_retry(1, &EventHandlerError::Retryable("x".into())));
    }

    #[test]
    fn retry_apenas_para_erro_retryable() {
        let policy = RetryPolicy::new(3);

        assert!(policy.should_retry(1, &EventHandlerError::Retryable("x".into())));
        assert!(policy.should_retry(2, &EventHandlerError::Retryable("x".into())));
        assert!(!policy.should_retry(3, &EventHandlerError::Retryable("x".into())));

        assert!(!policy.should_retry(1, &EventHandlerError::NonRetryable("x".into())));
        assert!(!policy.should_retry(1, &EventHandlerError::ProcessingFailed("x".into())));
    }

    #[test]
    fn zero_tentativas_e_elevado_a_um() {
        let policy = RetryPolicy::new(0);
        assert_eq!(policy.effective_max_attempts(), 1);
        assert!(!policy.should_retry(1, &EventHandlerError::Retryable("x".into())));
    }
}
