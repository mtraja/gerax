//! Adaptador da Persistent Subscription para o Event Relay.
//!
//! Este módulo só existe com a feature `eda`. Ele converte
//! [`SubscriptionEvent`] em [`IntegrationEvent`] e implementa a porta
//! genérica [`EventSource`] da `gerax-eda` — sem que os contratos
//! centrais da EDA dependam do cliente KurrentDB.
//!
//! A regra de confirmação é delegada ao relay: o ACK só é enviado
//! depois que o dispatcher (por exemplo, o publisher NATS/JetStream)
//! conclui com sucesso. Falhas de conversão de payload são tratadas
//! como permanentes e o evento é parkeado, evitando que uma mensagem
//! envenenada bloqueie o grupo.

use async_trait::async_trait;
use tracing::{debug, info, warn};

use gerax_eda::event_relay::{
    DeliveryAction, EventSource, IntegrationEvent, SourceError, SourceEvent,
};

use gerax_es::EventId;

use super::ack::SubscriptionAck;
use super::config::SubscriptionConfig;
use super::consumer::{KurrentDbSubscription, SubscriptionSession};
use super::error::SubscriptionError;
use super::event::SubscriptionEvent;
use super::idempotency::{SharedIdempotencyStore, IdempotencyStore};

/// Fonte de eventos KurrentDB para o Event Relay.
///
/// Mantém uma sessão ativa e reconecta automaticamente em falhas
/// transitórias. A reconexão respeita
/// [`SubscriptionConfig::reconnect_delay`].
///
/// Opcionalmente, uma [`IdempotencyStore`] rastreia os `event_id`
/// já despachados: reentregas do mesmo id são confirmadas com
/// [`SubscriptionAck::Skip`] sem que o relay as processe.
pub struct KurrentDbEventSource {
    subscription: KurrentDbSubscription,
    session: Option<SubscriptionSession>,
    idempotency: Option<SharedIdempotencyStore>,
}

impl KurrentDbEventSource {
    /// Cria a fonte a partir de um consumidor configurado.
    pub fn new(subscription: KurrentDbSubscription) -> Self {
        Self {
            subscription,
            session: None,
            idempotency: None,
        }
    }

    /// Associa uma [`IdempotencyStore`] ao fluxo de relay.
    ///
    /// Reentregas de `event_id` já despachados são ignoradas (ACK
    /// `Skip`) antes de chegar à camada superior.
    pub fn with_idempotency_store(
        mut self,
        store: impl IdempotencyStore + 'static,
    ) -> Self {
        self.idempotency = Some(super::idempotency::shared_idempotency_store(store));
        self
    }

    /// Associa uma [`SharedIdempotencyStore`] já existente.
    pub fn with_shared_idempotency_store(
        mut self,
        store: SharedIdempotencyStore,
    ) -> Self {
        self.idempotency = Some(store);
        self
    }

    /// Referência opcional à store de idempotência, se configurada.
    pub fn idempotency_store(&self) -> Option<&SharedIdempotencyStore> {
        self.idempotency.as_ref()
    }

    /// Configuração do grupo consumido.
    pub fn config(&self) -> &SubscriptionConfig {
        self.subscription.config()
    }

    /// Consumidor subjacente.
    pub fn subscription(&self) -> &KurrentDbSubscription {
        &self.subscription
    }

    /// Consome e devolve o consumidor, descartando a sessão ativa.
    pub fn into_subscription(self) -> KurrentDbSubscription {
        self.subscription
    }

    async fn session(&mut self) -> Result<&mut SubscriptionSession, SourceError> {
        if self.session.is_none() {
            let session = self
                .subscription
                .subscribe()
                .await
                .map_err(map_source_error)?;
            self.session = Some(session);
        }

        Ok(self
            .session
            .as_mut()
            .expect("sessão inicializada imediatamente acima"))
    }

    /// Confirma um evento como `Skip` na sessão ativa (usado para
    /// idempotência: evento já processado).
    async fn skip_event(&mut self, event: &SubscriptionEvent) -> Result<(), SourceError> {
        let acked = {
            let session = self.session().await?;
            session.ack(event, SubscriptionAck::Skip).await
        };
        acked.map_err(map_source_error)
    }
}

#[async_trait]
impl EventSource for KurrentDbEventSource {
    async fn next_event(&mut self) -> Result<SourceEvent, SourceError> {
        loop {
            let delay = self.subscription.config().reconnect_delay();

            let received = {
                let session = self.session().await?;
                session.next_event().await
            };

            match received {
                Ok(event) => {
                    // Idempotência: reentregas do mesmo event_id são
                    // ignoradas antes de chegar à camada superior.
                    if let Some(store) = &self.idempotency
                        && store.is_processed(&event.id).await
                    {
                        debug!(
                            metric = "kurrentdb.subscription.skip",
                            event_id = %event.id.as_uuid(),
                            subscription_group = %self.subscription.config().group,
                            stream = %event.stream_id,
                            "event_id já processado; confirmando com Skip"
                        );
                        if let Err(error) = self.skip_event(&event).await {
                            self.session = None;
                            return Err(error);
                        }
                        continue;
                    }

                    match subscription_event_to_integration(&event) {
                        Ok(integration) => {
                            return Ok(SourceEvent {
                                event: integration,
                                retry_count: event.retry_count,
                            });
                        }
                        Err(error) => {
                            warn!(
                                metric = "kurrentdb.subscription.park",
                                event_id = %event.id.as_uuid(),
                                event_type = %event.event_type,
                                aggregate_id = %event.aggregate_id,
                                stream_revision = event.stream_revision,
                                subscription_group = %self.subscription.config().group,
                                error = %error,
                                "evento inválido; encaminhando para park"
                            );

                            let parked = {
                                let session = self.session().await?;
                                session.ack(&event, SubscriptionAck::Park).await
                            };
                            if let Err(error) = parked {
                                self.session = None;
                                return Err(map_source_error(error));
                            }
                        }
                    }
                },
                Err(error) if error.is_retryable() => {
                    warn!(
                        error = %error,
                        delay_ms = delay.as_millis() as u64,
                        subscription_group = %self.subscription.config().group,
                        "conexão da subscription perdida; reconectando"
                    );
                    self.session = None;
                    tokio::time::sleep(delay).await;
                }
                Err(error) => {
                    self.session = None;
                    return Err(SourceError::Fatal(error.to_string()));
                }
            }
        }
    }

    async fn acknowledge(
        &mut self,
        event: &IntegrationEvent,
        action: DeliveryAction,
    ) -> Result<(), SourceError> {
        let ack = subscription_ack(action);
        let metric = match ack {
            SubscriptionAck::Ack => "kurrentdb.subscription.ack",
            SubscriptionAck::Retry => "kurrentdb.subscription.retry",
            SubscriptionAck::Park => "kurrentdb.subscription.park",
            SubscriptionAck::Skip => "kurrentdb.subscription.skip",
        };
        debug!(
            metric = metric,
            event_id = %event.id.as_uuid(),
            action = ?ack,
            subscription_group = %self.subscription.config().group,
            "confirmando evento no KurrentDB"
        );

        let result = {
            let session = self.session().await?;
            session
                .ack_id(
                    EventId::from_uuid(event.id.as_uuid()),
                    ack,
                    "gerax event relay",
                )
                .await
        };

        if result.is_ok()
            && ack.is_ack()
            && let Some(store) = &self.idempotency
        {
            store.mark_processed(EventId::from_uuid(event.id.as_uuid())).await;
        }

        result.map_err(|error| {
            let mapped = map_source_error(error);
            if mapped.is_retryable() {
                self.session = None;
            }
            mapped
        })
    }
}

/// Converte um [`SubscriptionEvent`] no envelope de integração da EDA.
///
/// Preserva o `event_id`, o tipo, o Aggregate, a revisão da stream, o
/// instante de ocorrência, correlação/causação e a metadata original.
pub fn subscription_event_to_integration(
    event: &SubscriptionEvent,
) -> Result<IntegrationEvent, SubscriptionError> {
    let stored = event.to_stored_event()?;
    let metadata = stored.metadata.clone();

    let correlation_id = metadata
        .get("correlation_id")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let causation_id = metadata
        .get("causation_id")
        .and_then(|value| value.as_str())
        .map(str::to_owned);

    Ok(IntegrationEvent {
        id: stored.event_id,
        event_type: stored.event_type,
        aggregate_id: stored.aggregate_id,
        aggregate_type: stored.aggregate_type,
        stream_revision: event.stream_revision,
        occurred_at: stored.occurred_at,
        correlation_id,
        causation_id,
        payload: stored.payload,
        metadata,
    })
}

/// Traduz a ação genérica do relay para a ação da subscription.
pub fn subscription_ack(action: DeliveryAction) -> SubscriptionAck {
    match action {
        DeliveryAction::Ack => SubscriptionAck::Ack,
        DeliveryAction::Retry => SubscriptionAck::Retry,
        DeliveryAction::Park => SubscriptionAck::Park,
        DeliveryAction::Skip => SubscriptionAck::Skip,
    }
}

fn map_source_error(error: SubscriptionError) -> SourceError {
    if error.is_retryable() {
        SourceError::Retryable(error.to_string())
    } else {
        SourceError::Fatal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;

    use gerax_es::event::EventId;
    use gerax_es::metadata::EventMetadata;
    use gerax_es::version::Version;

    use crate::event_store::mapper::KurrentEventMetadata;

    fn subscription_event() -> SubscriptionEvent {
        let mut metadata = EventMetadata::new();
        metadata.insert("correlation_id", "fluxo-1");
        metadata.insert("causation_id", "evt-0");
        metadata.insert("tenant", "cmf");

        let envelope = KurrentEventMetadata {
            aggregate_type: "Aluno".into(),
            aggregate_id: "aluno-1".into(),
            occurred_at: Utc::now(),
            metadata,
        };
        let metadata_bytes = serde_json::to_vec(&envelope).unwrap();

        SubscriptionEvent {
            id: EventId::new(),
            event_type: "AlunoMatriculado".into(),
            stream_id: "Aluno-aluno-1".into(),
            stream_revision: 4,
            payload: br#"{"aluno_id":"aluno-1"}"#.to_vec(),
            metadata: metadata_bytes,
            aggregate_type: "Aluno".into(),
            aggregate_id: "aluno-1".into(),
            occurred_at: envelope.occurred_at,
            retry_count: 2,
            commit_position: Some(10),
        }
    }

    #[test]
    fn conversao_preserva_identidade_e_revisao() {
        let source = subscription_event();
        let integration = subscription_event_to_integration(&source).unwrap();

        assert_eq!(integration.id, source.id);
        assert_eq!(integration.event_type, "AlunoMatriculado");
        assert_eq!(integration.aggregate_type, "Aluno");
        assert_eq!(integration.aggregate_id, "aluno-1");
        assert_eq!(integration.stream_revision, 4);
        assert_eq!(integration.occurred_at, source.occurred_at);
        assert_eq!(
            integration.payload,
            serde_json::json!({"aluno_id":"aluno-1"})
        );
        assert_eq!(integration.correlation_id.as_deref(), Some("fluxo-1"));
        assert_eq!(integration.causation_id.as_deref(), Some("evt-0"));
        assert_eq!(
            integration.metadata.get("tenant"),
            Some(&serde_json::Value::from("cmf"))
        );
        // versão Gerax derivada da revisão, sem perder a revisão original
        assert_eq!(integration.to_stored_event().version, Version::new(5));
    }

    #[test]
    fn payload_invalido_e_permanente() {
        let mut source = subscription_event();
        source.payload = b"nao-json".to_vec();
        let error = subscription_event_to_integration(&source).unwrap_err();
        assert!(error.is_permanent());
    }

    #[test]
    fn acoes_do_relay_mapeiam_para_ack_da_subscription() {
        assert_eq!(subscription_ack(DeliveryAction::Ack), SubscriptionAck::Ack);
        assert_eq!(
            subscription_ack(DeliveryAction::Retry),
            SubscriptionAck::Retry
        );
        assert_eq!(
            subscription_ack(DeliveryAction::Park),
            SubscriptionAck::Park
        );
        assert_eq!(
            subscription_ack(DeliveryAction::Skip),
            SubscriptionAck::Skip
        );
    }
}
