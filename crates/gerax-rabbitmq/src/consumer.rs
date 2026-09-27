//! Consumo de mensagens: registro de handlers, retry e DLQ (`§15`–`§19`).
//!
//! Fluxo do consumidor:
//!
//! ```text
//! RabbitMQ ──► Queue ──► deserialize (MessageEnvelope)
//!                            │
//!                            ▼
//!                    RabbitHandlerRegistry.get(event_type)
//!                            │
//!                            ▼
//!                  handler.handle(EventEnvelope<E>)
//!                            │
//!              ┌─────────────┼─────────────┐
//!              ▼             ▼             ▼
//!             Ok           retryável      não-retryável
//!              │            │              │
//!              ▼            ▼              ▼
//!            ACK        delay + republish  reject → DLQ
//!                           │
//!                           ▼
//!                      ACK da original
//! ```
//!
//! **At-least-once** (`§16`): ACK ocorre somente após o processamento
//! bem-sucedido. Retry **nunca** usa `requeue = true` infinito (`§17`):
//! a mensagem é re-publicada após um atraso configurável e a original é
//! então reconhecida (`§18`). Falhas finais rejeitam sem requeue → DLQ
//! quando configurada (`§19`).

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
use lapin::Channel;
use lapin::message::Delivery;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions, BasicRejectOptions};
use serde::de::DeserializeOwned;
use tracing::{debug, error, warn};

use crate::config::{DeliveryMode, RabbitConsumerConfig, UnknownEventPolicy};
use crate::connection::RabbitMqConnection;
use crate::envelope::{EnvelopeCodec, JsonEnvelopeCodec, MessageEnvelope, amqp_properties};
use crate::error::RabbitMqError;
use crate::topology;

/// Handler apagado para o consumidor RabbitMQ.
///
/// Recebe o [`MessageEnvelope`] já desserializado e devolve apenas o
/// erro classificado do [`gerax_eda::EventHandler`]; a decisão de
/// ack/retry/reject é do consumidor.
#[async_trait]
pub trait ErasedHandler: Send + Sync {
    /// Nome Rust do handler concreto (diagnóstico/observabilidade).
    fn name(&self) -> &'static str;

    /// Desserializa o payload e despacha para o handler tipado.
    async fn handle(&self, envelope: &MessageEnvelope) -> Result<(), EventHandlerError>;
}

/// Adapta um [`gerax_eda::EventHandler`] ao [`ErasedHandler`].
///
/// Constrói o [`EventEnvelope`] a partir do envelope de transporte e
/// delega ao handler tipado.
pub struct HandlerAdapter<H, E> {
    handler: H,
    _marker: PhantomData<fn() -> E>,
}

impl<H, E> HandlerAdapter<H, E> {
    fn new(handler: H) -> Self {
        Self {
            handler,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<H, E> ErasedHandler for HandlerAdapter<H, E>
where
    H: EventHandler<E> + Send + Sync,
    E: Event + DeserializeOwned,
{
    fn name(&self) -> &'static str {
        std::any::type_name::<H>()
    }

    async fn handle(&self, envelope: &MessageEnvelope) -> Result<(), EventHandlerError> {
        let event = envelope.payload::<E>().map_err(|source| {
            EventHandlerError::NonRetryable(format!(
                "decode payload as {} failed: {source}",
                std::any::type_name::<E>()
            ))
        })?;

        let event_envelope = EventEnvelope {
            id: envelope.event_id(),
            event,
            metadata: envelope.to_eda_metadata(),
        };

        self.handler.handle(event_envelope).await
    }
}

/// Registro de handlers por `event_type` (nome lógico do evento).
///
/// Thread-safe: permite registrar rotas em paralelo ao consumo.
/// Diferente do [`gerax_eda::EventHandlerRegistry`] (que roteia por
/// `TypeId`), aqui a chave é a **string** que chega no
/// [`MessageEnvelope::event_type`] — o contrato externo do evento.
#[derive(Default)]
pub struct RabbitHandlerRegistry {
    handlers: Mutex<HashMap<String, Arc<dyn ErasedHandler>>>,
}

impl RabbitHandlerRegistry {
    /// Cria um registro vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um handler para um `event_type`.
    ///
    /// `E` é inferido do handler (ex.: `AlunoMatriculado` de um
    /// `EventHandler<AlunoMatriculado>`).
    pub fn register<E, H>(&self, event_type: &str, handler: H)
    where
        E: Event + DeserializeOwned + 'static,
        H: EventHandler<E> + Send + Sync + 'static,
    {
        let adapter = HandlerAdapter::<H, E>::new(handler);
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .insert(event_type.to_string(), Arc::new(adapter));
    }

    /// Consulta o handler registrado para um `event_type`.
    pub(crate) fn get(&self, event_type: &str) -> Option<Arc<dyn ErasedHandler>> {
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .get(event_type)
            .cloned()
    }

    /// Lista os `event_type` registrados (ordenados).
    pub fn event_types(&self) -> Vec<String> {
        let mut keys = self
            .handlers
            .lock()
            .expect("handler registry lock poisoned")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    /// `true` quando há handler para o `event_type`.
    pub fn contains(&self, event_type: &str) -> bool {
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .contains_key(event_type)
    }

    /// Número de rotas registradas.
    pub fn len(&self) -> usize {
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .len()
    }

    /// `true` quando não há rotas registradas.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Consome a fila e entrega cada mensagem ao handler registrado para
/// o seu `event_type`.
pub struct RabbitMqConsumer<C: EnvelopeCodec = JsonEnvelopeCodec> {
    /// Exchange de eventos usado (e no republish do retry).
    exchange: String,
    /// Modo de entrega das mensagens (e no republish do retry).
    delivery_mode: DeliveryMode,
    channel: Channel,
    registry: Arc<RabbitHandlerRegistry>,
    config: RabbitConsumerConfig,
    queue: String,
    codec: C,
}

impl RabbitMqConsumer<JsonEnvelopeCodec> {
    /// Conecta, declara exchange/fila/DLQ, liga a fila e ajusta o
    /// prefetch, usando o codec JSON padrão.
    pub async fn connect(
        connection: &RabbitMqConnection,
        config: RabbitConsumerConfig,
        registry: Arc<RabbitHandlerRegistry>,
    ) -> Result<Self, RabbitMqError> {
        Self::connect_with_codec(connection, config, registry, JsonEnvelopeCodec).await
    }
}

impl<C: EnvelopeCodec> RabbitMqConsumer<C> {
    /// Conecta usando um codec custom.
    pub async fn connect_with_codec(
        connection: &RabbitMqConnection,
        config: RabbitConsumerConfig,
        registry: Arc<RabbitHandlerRegistry>,
        codec: C,
    ) -> Result<Self, RabbitMqError> {
        let channel = connection.channel().await?;
        let connection_config = connection.config();

        topology::declare_exchange(&channel, connection_config)
            .await
            .map_err(RabbitMqError::from)?;
        topology::declare_dlx_and_dlq(&channel, &config.dead_letter)
            .await
            .map_err(RabbitMqError::from)?;

        let queue = topology::declare_queue(&channel, &config).await?;

        let binding_keys = if config.bindings.is_empty() {
            registry.event_types()
        } else {
            config.bindings.clone()
        };
        if binding_keys.is_empty() {
            warn!(
                queue = %config.queue,
                "consumer has no bindings: register handlers or configure bindings",
            );
        }
        topology::bind_queue(&channel, &queue, &connection_config.exchange, &binding_keys)
            .await
            .map_err(RabbitMqError::from)?;

        channel
            .basic_qos(config.prefetch_count, BasicQosOptions::default())
            .await?;

        debug!(
            queue = %config.queue,
            bindings = %binding_keys.join(","),
            prefetch = %config.prefetch_count,
            "consumer ready",
        );

        Ok(Self {
            exchange: connection_config.exchange.clone(),
            delivery_mode: connection_config.delivery_mode,
            channel,
            registry,
            config,
            queue,
            codec,
        })
    }

    /// Registro usado por este consumidor.
    pub fn registry(&self) -> &RabbitHandlerRegistry {
        &self.registry
    }

    /// Fila consumida por este consumidor.
    pub fn queue(&self) -> &str {
        &self.queue
    }

    /// Consome a fila até o canal fechar ou ocorrer um erro.
    ///
    /// Acknowledgment (semântica **at-least-once**):
    ///
    /// - `Ok` → `ack`;
    /// - [`EventHandlerError::Retryable`] abaixo do
    ///   `retry.max_attempts` → re-publica após atraso exponencial e
    ///   então `ack` da original;
    /// - [`EventHandlerError::NonRetryable`] ou retry esgotado →
    ///   `reject` (→ DLQ quando configurada);
    /// - payload malformado → `reject` (→ DLQ quando configurada);
    /// - evento sem handler → política `on_unknown_event`.
    ///
    /// Em caso de erro AMQP a função retorna e o chamador decide como
    /// respawnar.
    pub async fn consume(&self) -> Result<(), RabbitMqError> {
        let mut consumer = self
            .channel
            .basic_consume(
                self.queue.clone().into(),
                self.config.consumer_tag.clone().into(),
                BasicConsumeOptions::default(),
                Default::default(),
            )
            .await?;

        debug!(
            queue = %self.queue,
            consumer_tag = %consumer.tag(),
            "consumer started",
        );

        while let Some(delivery) = consumer.next().await {
            let delivery = delivery?;
            self.deliver(&delivery).await;
        }

        Ok(())
    }

    /// Roteia uma entrega para o handler e aplica a política de ack.
    async fn deliver(&self, delivery: &Delivery) {
        let envelope = match self.codec.decode(&delivery.data) {
            Ok(envelope) => envelope,
            Err(source) => {
                error!(
                    queue = %self.queue,
                    error = %source,
                    "malformed message, rejecting",
                );
                self.reject(delivery).await;
                return;
            }
        };

        let event_type = envelope.event_type().to_string();
        let attempts = envelope.attempts();

        let Some(handler) = self.registry.get(&event_type) else {
            warn!(
                event_type,
                attempts,
                queue = %self.queue,
                "delivery has no registered handler",
            );
            match self.config.on_unknown_event {
                UnknownEventPolicy::Ack => self.ack(delivery).await,
                UnknownEventPolicy::Reject => self.reject(delivery).await,
            }
            return;
        };

        match handler.handle(&envelope).await {
            Ok(()) => {
                debug!(
                    event_type,
                    handler = %handler.name(),
                    attempts,
                    "event handled, acking",
                );
                self.ack(delivery).await;
            }
            Err(failure)
                if failure.is_retryable() && self.config.retry.should_retry(attempts + 1) =>
            {
                let next_attempt = attempts + 1;
                warn!(
                    event_type,
                    handler = %handler.name(),
                    attempt = %next_attempt,
                    max = %self.config.retry.effective_max_attempts(),
                    error = %failure,
                    "handler failed retryably, will retry",
                );
                self.retry_with_delay(&envelope, delivery, next_attempt)
                    .await;
            }
            Err(failure) => {
                error!(
                    event_type,
                    handler = %handler.name(),
                    attempts,
                    error = %failure,
                    "handler failed, rejecting",
                );
                self.reject(delivery).await;
            }
        }
    }

    /// Re-publica a mensagem após o atraso exponencial (sem
    /// `requeue = true`) e só então reconhece a original (`§17`/`§18`).
    async fn retry_with_delay(
        &self,
        envelope: &MessageEnvelope,
        delivery: &Delivery,
        attempt: u32,
    ) {
        let delay = self.config.retry.delay_for(attempt);
        tokio::time::sleep(delay).await;

        let mut retried = envelope.clone();
        retried.set_attempts(attempt);

        let payload = match self.codec.encode(&retried) {
            Ok(payload) => payload,
            Err(source) => {
                error!(
                    event_id = %retried.event_id().as_uuid(),
                    error = %source,
                    "cannot re-encode message for retry",
                );
                self.reject(delivery).await;
                return;
            }
        };

        let properties = amqp_properties(&retried, self.delivery_mode);
        let confirm = match self
            .channel
            .basic_publish(
                self.exchange.clone().into(),
                retried.event_type().to_string().into(),
                Default::default(),
                &payload,
                properties,
            )
            .await
        {
            Ok(confirm) => confirm.await,
            Err(source) => {
                error!(
                    event_id = %retried.event_id().as_uuid(),
                    error = ?source,
                    "failed to republish retry, rejecting original",
                );
                self.reject(delivery).await;
                return;
            }
        };

        match confirm {
            Ok(lapin::Confirmation::Ack(_)) | Ok(lapin::Confirmation::NotRequested) => {
                debug!(
                    event_id = %retried.event_id().as_uuid(),
                    event_type = %retried.event_type(),
                    attempt,
                    "retried message republished, acking original",
                );
                self.ack(delivery).await;
            }
            Ok(lapin::Confirmation::Nack(_)) => {
                error!(
                    event_id = %retried.event_id().as_uuid(),
                    "broker nacked the republished retry, rejecting original",
                );
                self.reject(delivery).await;
            }
            Err(source) => {
                error!(
                    event_id = %retried.event_id().as_uuid(),
                    error = ?source,
                    "failed to confirm republished retry, rejecting original",
                );
                self.reject(delivery).await;
            }
        }
    }

    async fn ack(&self, delivery: &Delivery) {
        if let Err(source) = delivery.ack(BasicAckOptions::default()).await {
            error!(queue = %self.queue, error = %source, "failed to ack delivery");
        }
    }

    /// Rejeita sem requeue: a mensagem vai para a DLQ quando a topologia
    /// de dead-letter está configurada, caso contrário é descartada.
    async fn reject(&self, delivery: &Delivery) {
        if let Err(source) = delivery.reject(BasicRejectOptions { requeue: false }).await {
            error!(queue = %self.queue, error = %source, "failed to reject delivery");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_eda::{CausationId, CorrelationId};

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[derive(Debug, Default)]
    struct RegistrarAluno;

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for RegistrarAluno {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            Ok(())
        }
    }

    fn envelope() -> MessageEnvelope {
        let event_envelope = EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        })
        .with_correlation_id("fluxo-1")
        .with_causation_id("evt-0")
        .with_header("tenant", "cmf");
        MessageEnvelope::from_event(&event_envelope).expect("envelope valido")
    }

    #[test]
    fn registry_roteria_por_event_type() {
        let registry = RabbitHandlerRegistry::new();
        registry.register::<AlunoMatriculado, _>("aluno.matriculado", RegistrarAluno);

        assert!(registry.contains("aluno.matriculado"));
        assert!(!registry.contains("aluno.cancelado"));
        assert_eq!(
            registry.event_types(),
            vec!["aluno.matriculado".to_string()]
        );
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let handler = registry.get("aluno.matriculado").unwrap();
        assert!(handler.name().contains("RegistrarAluno"));
    }

    #[tokio::test]
    async fn adapter_converte_envelope_de_transporte_para_eda() {
        let adapter = Arc::new(HandlerAdapter::<RegistrarAluno, AlunoMatriculado>::new(
            RegistrarAluno,
        ));
        let envelope = envelope();

        // o handler registrado no registry recebe o envelope tipado
        assert!(adapter.handle(&envelope).await.is_ok());
        let event = envelope.payload::<AlunoMatriculado>().unwrap();
        assert_eq!(event.aluno_id, "a-1");
    }

    #[test]
    fn envelope_do_teste_carrega_rastreabilidade() {
        let envelope = envelope();
        assert_eq!(envelope.correlation_id.as_deref(), Some("fluxo-1"));
        assert_eq!(envelope.causation_id.as_deref(), Some("evt-0"));
        assert_eq!(
            envelope.headers.get("tenant").map(String::as_str),
            Some("cmf")
        );

        let metadata = envelope.to_eda_metadata();
        assert_eq!(
            metadata.correlation_id.as_ref().map(CorrelationId::as_str),
            Some("fluxo-1")
        );
        assert_eq!(
            metadata.causation_id.as_ref().map(CausationId::as_str),
            Some("evt-0")
        );
    }
}
