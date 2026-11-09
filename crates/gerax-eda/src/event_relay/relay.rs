//! Orquestrador do Event Relay.
//!
//! O relay liga uma [`EventSource`] (por exemplo, Persistent
//! Subscription do KurrentDB) a um [`EventDispatch`] (por exemplo,
//! publicação no NATS JetStream via `gerax-nats`):
//!
//! ```text
//! next_event() ──► dispatch() ──► sucesso? ──► acknowledge(Ack)
//!                                    │
//!                                    ├── falha transitória ──► Retry/Park
//!                                    └── falha permanente  ──► Park/Skip
//! ```
//!
//! A regra fundamental: **o ACK na origem só acontece depois que o
//! dispatch retorna sucesso**. Se a publicação falhar, a origem não é
//! confirmada e o evento será reentregue.
//!
//! # At-least-once
//!
//! O relay não promete exactly-once entre origem e destino. Se o
//! processo falhar entre o dispatch e o ACK, a origem reentrega o
//! evento e ele pode ser publicado novamente. Consumidores devem ser
//! idempotentes (ver [`crate::EventId`] e `Nats-Msg-Id`).

use thiserror::Error;
use tracing::{debug, error, info, warn};

use super::envelope::IntegrationEvent;
use super::handler::HandlerError;
use super::source::{DeliveryAction, EventSource, SourceError, SourceEvent};

/// Ação tomada quando o dispatch falha de forma permanente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermanentFailureAction {
    /// Enviar o evento para park (dead-letter) na origem.
    Park,
    /// Ignorar o evento sem park e sem reentrega.
    Skip,
}

/// Política de retry do relay.
///
/// `max_retry_count` limita quantas vezes uma falha transitória pode
/// ser reentregue antes de o evento ir para park. O valor `0` desativa
/// o retry: a primeira falha transitória já vai para park.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayRetryPolicy {
    /// Máximo de reentregas por falha transitória.
    pub max_retry_count: u32,
    /// Ação aplicada a falhas permanentes.
    pub permanent_failure: PermanentFailureAction,
}

impl RelayRetryPolicy {
    /// Cria uma política com o máximo de reentregas indicado.
    pub const fn new(max_retry_count: u32) -> Self {
        Self {
            max_retry_count,
            permanent_failure: PermanentFailureAction::Park,
        }
    }

    /// Define a ação para falhas permanentes (builder).
    pub const fn with_permanent_failure(mut self, action: PermanentFailureAction) -> Self {
        self.permanent_failure = action;
        self
    }

    /// `true` quando a falha transitória ainda pode ser retentada.
    pub const fn should_retry(&self, retry_count: u32) -> bool {
        retry_count < self.max_retry_count
    }

    /// Decide a ação para um erro de processamento.
    pub const fn action_for(&self, error: &HandlerError, retry_count: u32) -> DeliveryAction {
        match error {
            HandlerError::Retryable(_) => {
                if self.should_retry(retry_count) {
                    DeliveryAction::Retry
                } else {
                    DeliveryAction::Park
                }
            }
            HandlerError::Permanent(_) => match self.permanent_failure {
                PermanentFailureAction::Park => DeliveryAction::Park,
                PermanentFailureAction::Skip => DeliveryAction::Skip,
            },
        }
    }
}

impl Default for RelayRetryPolicy {
    fn default() -> Self {
        Self::new(3)
    }
}

/// Contadores acumulados de uma execução do relay.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RelayStats {
    /// Eventos recebidos da origem.
    pub received: u64,
    /// Eventos confirmados com sucesso.
    pub acked: u64,
    /// Eventos devolvidos à origem para reentrega.
    pub retried: u64,
    /// Eventos enviados para park.
    pub parked: u64,
    /// Eventos ignorados.
    pub skipped: u64,
}

/// Resultado de um passo do relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayOutcome {
    /// Evento processado e confirmado com a ação indicada.
    Processed(DeliveryAction),
    /// A origem terminou normalmente.
    Closed,
}

/// Erros fatais do relay.
#[derive(Debug, Error)]
pub enum RelayError {
    /// Falha ao obter o próximo evento.
    #[error("event source error: {0}")]
    Source(#[source] SourceError),

    /// Falha ao confirmar o evento na origem.
    #[error("event acknowledge error: {0}")]
    Acknowledge(#[source] SourceError),
}

/// Orquestrador genérico de eventos.
///
/// Genérico sobre a origem e o dispatcher para não acoplar a EDA a
/// nenhum transporte. O relay pode ser dirigido passo a passo
/// ([`EventRelay::step`]) ou até o fecho da origem
/// ([`EventRelay::run`]).
pub struct EventRelay<S, D> {
    source: S,
    dispatcher: D,
    retry: RelayRetryPolicy,
    stats: RelayStats,
}

impl<S, D> EventRelay<S, D> {
    /// Cria o relay.
    pub fn new(source: S, dispatcher: D) -> Self {
        Self {
            source,
            dispatcher,
            retry: RelayRetryPolicy::default(),
            stats: RelayStats::default(),
        }
    }

    /// Define a política de retry (builder).
    pub fn with_retry_policy(mut self, retry: RelayRetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Política de retry ativa.
    pub const fn retry_policy(&self) -> RelayRetryPolicy {
        self.retry
    }

    /// Origem de eventos.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Origem mutável (introspeção/testes).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Dispatcher usado no processamento.
    pub fn dispatcher(&self) -> &D {
        &self.dispatcher
    }

    /// Consome o relay e devolve a origem (após o loop/step).
    ///
    /// Útil para inspeção ou para reconectar uma nova sessão de teste.
    pub fn into_source(self) -> S {
        self.source
    }

    /// Consome o relay e devolve o dispatcher.
    pub fn into_dispatcher(self) -> D {
        self.dispatcher
    }

    /// Contadores acumulados.
    pub const fn stats(&self) -> RelayStats {
        self.stats
    }
}

impl<S, D> EventRelay<S, D>
where
    S: EventSource,
    D: super::dispatcher::EventDispatch,
{
    /// Processa um único evento.
    ///
    /// Devolve [`RelayOutcome::Closed`] quando a origem termina
    /// normalmente. Erros de leitura ou de confirmação são fatais para
    /// o passo e interrompem a execução — a origem decide se deve
    /// reconectar na próxima chamada.
    pub async fn step(&mut self) -> Result<RelayOutcome, RelayError> {
        let source_event = match self.source.next_event().await {
            Ok(event) => event,
            Err(SourceError::Closed) => {
                debug!("event source closed");
                return Ok(RelayOutcome::Closed);
            }
            Err(error) => return Err(RelayError::Source(error)),
        };
        self.stats.received += 1;

        let action = self.decide(&source_event).await;
        self.record(action, &source_event.event);

        match &action {
            DeliveryAction::Ack => self.stats.acked += 1,
            DeliveryAction::Retry => self.stats.retried += 1,
            DeliveryAction::Park => self.stats.parked += 1,
            DeliveryAction::Skip => self.stats.skipped += 1,
        }

        self.source
            .acknowledge(&source_event.event, action)
            .await
            .map_err(RelayError::Acknowledge)?;

        Ok(RelayOutcome::Processed(action))
    }

    /// Executa até a origem fechar.
    pub async fn run(&mut self) -> Result<RelayStats, RelayError> {
        loop {
            if let RelayOutcome::Closed = self.step().await? {
                return Ok(self.stats);
            }
        }
    }

    async fn decide(&self, source_event: &SourceEvent) -> DeliveryAction {
        match self.dispatcher.dispatch(&source_event.event).await {
            Ok(()) => DeliveryAction::Ack,
            Err(error) => {
                let action = self.retry.action_for(&error, source_event.retry_count);
                self.log_failure(source_event, &error, action);
                action
            }
        }
    }

    fn log_failure(
        &self,
        source_event: &SourceEvent,
        error: &HandlerError,
        action: DeliveryAction,
    ) {
        let event = &source_event.event;
        match action {
            DeliveryAction::Retry => warn!(
                event_id = %event.id.as_uuid(),
                event_type = %event.event_type,
                aggregate_id = %event.aggregate_id,
                retry_count = source_event.retry_count,
                max_retry_count = self.retry.max_retry_count,
                error = %error,
                "falha transitória; solicitando reentrega"
            ),
            DeliveryAction::Park => warn!(
                event_id = %event.id.as_uuid(),
                event_type = %event.event_type,
                aggregate_id = %event.aggregate_id,
                retry_count = source_event.retry_count,
                error = %error,
                reason = ?error,
                "evento encaminhado para park"
            ),
            DeliveryAction::Skip => warn!(
                event_id = %event.id.as_uuid(),
                event_type = %event.event_type,
                error = %error,
                "evento ignorado por política"
            ),
            DeliveryAction::Ack => {}
        }
    }

    fn record(&self, action: DeliveryAction, event: &IntegrationEvent) {
        match action {
            DeliveryAction::Ack => info!(
                event_id = %event.id.as_uuid(),
                event_type = %event.event_type,
                aggregate_id = %event.aggregate_id,
                stream_revision = event.stream_revision,
                action = "ack",
                "evento processado e confirmado"
            ),

            DeliveryAction::Retry => warn!(
                event_id = %event.id.as_uuid(),
                event_type = %event.event_type,
                action = "retry",
                "evento será reprocessado"
            ),

            DeliveryAction::Park => error!(
                event_id = %event.id.as_uuid(),
                event_type = %event.event_type,
                action = "park",
                "evento enviado para tratamento posterior"
            ),

            DeliveryAction::Skip => warn!(
                event_id = %event.id.as_uuid(),
                event_type = %event.event_type,
                action = "skip",
                "evento ignorado"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::VecDeque;

    use async_trait::async_trait;
    use chrono::Utc;
    use gerax_es::EventId;

    use super::super::dispatcher::{EventDispatch, EventDispatcher};
    use super::super::handler::IntegrationEventHandler;

    fn event(event_type: &str) -> IntegrationEvent {
        IntegrationEvent::new(
            EventId::new(),
            event_type,
            "aluno-1",
            "Aluno",
            0,
            Utc::now(),
            serde_json::json!({}),
        )
    }

    struct ScriptedSource {
        events: VecDeque<SourceEvent>,
        actions: Vec<DeliveryAction>,
    }

    impl ScriptedSource {
        fn new(events: Vec<SourceEvent>) -> Self {
            Self {
                events: events.into(),
                actions: Vec::new(),
            }
        }
    }

    #[async_trait]
    impl EventSource for ScriptedSource {
        async fn next_event(&mut self) -> Result<SourceEvent, SourceError> {
            self.events.pop_front().ok_or(SourceError::Closed)
        }

        async fn acknowledge(
            &mut self,
            _event: &IntegrationEvent,
            action: DeliveryAction,
        ) -> Result<(), SourceError> {
            self.actions.push(action);
            Ok(())
        }
    }

    struct FixedHandler {
        result: Result<(), HandlerError>,
    }

    #[async_trait]
    impl IntegrationEventHandler for FixedHandler {
        async fn handle(&self, _event: &IntegrationEvent) -> Result<(), HandlerError> {
            self.result.clone()
        }
    }

    fn dispatcher_with(result: Result<(), HandlerError>) -> EventDispatcher {
        EventDispatcher::new().register("AlunoMatriculado", FixedHandler { result })
    }

    #[tokio::test]
    async fn sucesso_envia_ack() {
        let source = ScriptedSource::new(vec![SourceEvent::new(event("AlunoMatriculado"))]);
        let mut relay = EventRelay::new(source, dispatcher_with(Ok(())));

        let outcome = relay.step().await.unwrap();

        assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Ack));
        assert_eq!(relay.source().actions, vec![DeliveryAction::Ack]);
        assert_eq!(relay.stats().acked, 1);
    }

    #[tokio::test]
    async fn falha_transitoria_reentrega_antes_do_limite() {
        let source = ScriptedSource::new(vec![
            SourceEvent::new(event("AlunoMatriculado")).with_retry_count(0),
        ]);
        let mut relay = EventRelay::new(
            source,
            dispatcher_with(Err(HandlerError::retryable("offline"))),
        )
        .with_retry_policy(RelayRetryPolicy::new(2));

        let outcome = relay.step().await.unwrap();

        assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Retry));
        assert_eq!(relay.source().actions, vec![DeliveryAction::Retry]);
        assert_eq!(relay.stats().retried, 1);
    }

    #[tokio::test]
    async fn falha_transitoria_apos_limite_vai_para_park() {
        let source = ScriptedSource::new(vec![
            SourceEvent::new(event("AlunoMatriculado")).with_retry_count(2),
        ]);
        let mut relay = EventRelay::new(
            source,
            dispatcher_with(Err(HandlerError::retryable("offline"))),
        )
        .with_retry_policy(RelayRetryPolicy::new(2));

        let outcome = relay.step().await.unwrap();

        assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Park));
        assert_eq!(relay.source().actions, vec![DeliveryAction::Park]);
        assert_eq!(relay.stats().parked, 1);
    }

    #[tokio::test]
    async fn falha_permanente_vai_para_park_por_padrao() {
        let source = ScriptedSource::new(vec![SourceEvent::new(event("AlunoMatriculado"))]);
        let mut relay = EventRelay::new(
            source,
            dispatcher_with(Err(HandlerError::permanent("invalid"))),
        );

        let outcome = relay.step().await.unwrap();

        assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Park));
        assert_eq!(relay.source().actions, vec![DeliveryAction::Park]);
    }

    #[tokio::test]
    async fn politica_pode_ignorar_falha_permanente() {
        let source = ScriptedSource::new(vec![SourceEvent::new(event("AlunoMatriculado"))]);
        let mut relay = EventRelay::new(
            source,
            dispatcher_with(Err(HandlerError::permanent("invalid"))),
        )
        .with_retry_policy(
            RelayRetryPolicy::new(2).with_permanent_failure(PermanentFailureAction::Skip),
        );

        let outcome = relay.step().await.unwrap();

        assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Skip));
        assert_eq!(relay.source().actions, vec![DeliveryAction::Skip]);
        assert_eq!(relay.stats().skipped, 1);
    }

    #[tokio::test]
    async fn tipo_sem_handler_nao_e_confirmado_com_ack() {
        let source = ScriptedSource::new(vec![SourceEvent::new(event("Desconhecido"))]);
        let mut relay = EventRelay::new(source, EventDispatcher::new());

        let outcome = relay.step().await.unwrap();

        assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Park));
        assert_eq!(relay.stats().parked, 1);
    }

    #[tokio::test]
    async fn run_processa_ate_o_fecho() {
        let source = ScriptedSource::new(vec![
            SourceEvent::new(event("AlunoMatriculado")),
            SourceEvent::new(event("AlunoMatriculado")),
        ]);
        let mut relay = EventRelay::new(source, dispatcher_with(Ok(())));

        let stats = relay.run().await.unwrap();

        assert_eq!(stats.received, 2);
        assert_eq!(stats.acked, 2);
        assert_eq!(
            relay.source().actions,
            vec![DeliveryAction::Ack, DeliveryAction::Ack]
        );
    }

    #[tokio::test]
    async fn erro_fatal_da_origem_interrompe() {
        struct FailingSource;

        #[async_trait]
        impl EventSource for FailingSource {
            async fn next_event(&mut self) -> Result<SourceEvent, SourceError> {
                Err(SourceError::Fatal("boom".into()))
            }

            async fn acknowledge(
                &mut self,
                _event: &IntegrationEvent,
                _action: DeliveryAction,
            ) -> Result<(), SourceError> {
                Ok(())
            }
        }

        let mut relay = EventRelay::new(FailingSource, EventDispatcher::new());
        let error = relay.step().await.unwrap_err();
        assert!(matches!(error, RelayError::Source(_)));
    }

    #[tokio::test]
    async fn dispatcher_customizado_e_aceito() {
        struct AlwaysOk;

        #[async_trait]
        impl EventDispatch for AlwaysOk {
            async fn dispatch(&self, _event: &IntegrationEvent) -> Result<(), HandlerError> {
                Ok(())
            }
        }

        let source = ScriptedSource::new(vec![SourceEvent::new(event("Qualquer"))]);
        let mut relay = EventRelay::new(source, AlwaysOk);
        let outcome = relay.step().await.unwrap();
        assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Ack));
    }
}
