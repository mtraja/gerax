//! Consumo JetStream: consumidor durável, ACK, redelivery e DLQ
//! (`§12`–`§15`) — feature `jetstream`.
//!
//! Fluxo do consumidor:
//!
//! ```text
//! fetch ──► decode (NatsEnvelope) ──► handler ──► Ok: double-ACK
//!                     │                    │
//!               inválido/excedeu      falha: no-ACK
//!                     │                    │
//!                     ▼                    ▼
//!               DLQ (poison)      JetStream redelivery
//! ```
//!
//! - **ACK após processamento** (`§12`): o ACK só acontece depois do
//!   handler retornar `Ok` — nunca antes do processamento;
//! - **redelivery é do JetStream** (`§13`): falha do handler → no-ACK →
//!   o servidor reentrega após `ack_wait`, até `max_deliver`; **não**
//!   há retry manual em loop;
//! - **replay** (`§28`): com `replay_from_start` (padrão), o consumer
//!   durável entrega o histórico da stream (`DeliverPolicy::Ack`);
//! - **DLQ / poison** (`§14`): payload indecifrável ou falhas além de
//!   `max_deliver` são encaminhados a `<prefixo>.dlq.<service>.<event>`
//!   com o payload original, erro, tentativa e timestamp — nunca
//!   descarte silencioso;
//! - **idempotência** (`§15`): entregas podem se repetir (at-least-once);
//!   consumidores devem deduplicar por `event_id` (o header
//!   `Nats-Msg-Id` + `duplicate_window` do JetStream já deduplicam
//!   republicações do produtor).

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt;
use gerax_eda::{Event, EventEnvelope, EventHandler};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::client::NatsClient;
use crate::envelope::{self, NatsEnvelope};
use crate::error::NatsError;
use crate::subject::SubjectBuilder;

/// Timeout padrão de `ack_wait` (redelivery).
pub const DEFAULT_ACK_WAIT: Duration = Duration::from_secs(30);
/// Entregas padrão antes de tratar como poison.
pub const DEFAULT_MAX_DELIVER: u64 = 5;
/// Tamanho padrão do batch de `fetch`.
pub const DEFAULT_BATCH_SIZE: usize = 32;

/// Configuração de dead-letter (`§14`).
///
/// Mensagens poison são publicadas em
/// `<prefixo>.dlq.<service>.<event_type>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlqConfig {
    /// Nome do serviço consumidor (parte do subject da DLQ).
    pub service: String,
}

impl DlqConfig {
    /// Configura DLQ para o serviço.
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}

/// Configuração do consumidor durável.
#[derive(Debug, Clone, PartialEq)]
pub struct NatsConsumerConfig {
    /// Stream JetStream consumida.
    pub stream: String,
    /// Nome do consumer durável (um por serviço — `§12`).
    pub durable_name: String,
    /// Subjects filtrados (padrão: `["<prefixo>.>"]`).
    pub filter_subjects: Vec<String>,
    /// Política de ACK (sempre explícita — `§12`).
    pub ack_wait: Duration,
    /// Entregas máximas antes de poison → DLQ.
    pub max_deliver: u64,
    /// Tamanho do batch de fetch.
    pub batch_size: usize,
    /// `true` (padrão) entrega o histórico da stream (replay).
    pub replay_from_start: bool,
    /// Dead-letter; `None` → poison é confirmado com log de erro.
    pub dlq: Option<DlqConfig>,
}

impl NatsConsumerConfig {
    /// Cria a configuração com `durable_name` e os demais padrões.
    pub fn new(durable_name: impl Into<String>) -> Self {
        Self {
            stream: crate::config::DEFAULT_STREAM_NAME.into(),
            durable_name: durable_name.into(),
            filter_subjects: Vec::new(),
            ack_wait: DEFAULT_ACK_WAIT,
            max_deliver: DEFAULT_MAX_DELIVER,
            batch_size: DEFAULT_BATCH_SIZE,
            replay_from_start: true,
            dlq: None,
        }
    }

    /// Builder fluente.
    pub fn builder() -> NatsConsumerConfigBuilder {
        NatsConsumerConfigBuilder::new()
    }
}

impl Default for NatsConsumerConfig {
    fn default() -> Self {
        Self::new("gerax-consumer")
    }
}

/// Builder fluente de [`NatsConsumerConfig`].
pub struct NatsConsumerConfigBuilder {
    config: NatsConsumerConfig,
}

impl NatsConsumerConfigBuilder {
    /// Builder com os valores padrão.
    pub fn new() -> Self {
        Self {
            config: NatsConsumerConfig::default(),
        }
    }

    /// Define a stream.
    pub fn stream(mut self, stream: impl Into<String>) -> Self {
        self.config.stream = stream.into();
        self
    }

    /// Define o nome do consumer durável.
    pub fn durable_name(mut self, name: impl Into<String>) -> Self {
        self.config.durable_name = name.into();
        self
    }

    /// Define os subjects filtrados.
    pub fn filter_subjects(
        mut self,
        subjects: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.config.filter_subjects = subjects.into_iter().map(Into::into).collect();
        self
    }

    /// Define o `ack_wait` (redelivery após o intervalo).
    pub fn ack_wait(mut self, ack_wait: Duration) -> Self {
        self.config.ack_wait = ack_wait;
        self
    }

    /// Define o máximo de entregas antes de poison.
    pub fn max_deliver(mut self, max_deliver: u64) -> Self {
        self.config.max_deliver = if max_deliver == 0 { 1 } else { max_deliver };
        self
    }

    /// Define o tamanho do batch de fetch.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.config.batch_size = batch_size.max(1);
        self
    }

    /// Habilita/desabilita replay do histórico (`DeliverPolicy`).
    pub fn replay_from_start(mut self, replay: bool) -> Self {
        self.config.replay_from_start = replay;
        self
    }

    /// Habilita a DLQ para o serviço.
    pub fn dead_letter(mut self, service: impl Into<String>) -> Self {
        self.config.dlq = Some(DlqConfig::new(service));
        self
    }

    /// Constrói validando a configuração.
    pub fn build(self) -> Result<NatsConsumerConfig, NatsError> {
        let config = self.config;
        if config.durable_name.trim().is_empty() {
            return Err(NatsError::configuration(
                "durable consumer name must not be empty",
            ));
        }
        if config.stream.trim().is_empty() {
            return Err(NatsError::configuration("stream name must not be empty"));
        }
        Ok(config)
    }
}

impl Default for NatsConsumerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Mensagem já entregue pelo JetStream, com a tentativa atual.
#[derive(Debug, Clone)]
pub struct NatsDelivered {
    /// Envelope de transporte decodificado.
    pub envelope: NatsEnvelope,
    /// Subject original da mensagem.
    pub subject: String,
    /// Stream de origem.
    pub stream: String,
    /// Consumer que entregou.
    pub consumer: String,
    /// Número da entrega (1 = primeira tentativa).
    pub delivered: u64,
    /// Sequência na stream.
    pub stream_sequence: u64,
}

/// Mensagem publicada na dead-letter (`§14`).
///
/// Preserva o payload original e todo o contexto da falha.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DlqMessage {
    /// `event_id` original (quando decifrável).
    pub event_id: Option<String>,
    /// `event_type` original (ou `"unknown"`).
    pub event_type: String,
    /// Subject original da mensagem.
    pub subject: String,
    /// Número de entregas realizadas.
    pub delivered: u64,
    /// Erro que originou o encaminhamento.
    pub error: String,
    /// Instante do encaminhamento.
    pub failed_at: chrono::DateTime<chrono::Utc>,
    /// Correlação original, se conhecida.
    pub correlation_id: Option<String>,
    /// Causação original, se conhecida.
    pub causation_id: Option<String>,
    /// Payload original (ou `null` quando indecifrável).
    pub payload: Value,
}

/// Consumidor durável do JetStream.
///
/// Seguro para concorrência; múltiplas instâncias do mesmo
/// `durable_name` **distribuem** as mensagens entre si (competição
/// saudável de um mesmo consumer durável).
pub struct NatsConsumer {
    client: NatsClient,
    config: NatsConsumerConfig,
    subjects: SubjectBuilder,
}

impl NatsConsumer {
    /// Cria o consumidor a partir da conexão e da configuração.
    ///
    /// Exige a feature `jetstream` habilitada na `NatsConfig`.
    pub fn new(client: &NatsClient, config: NatsConsumerConfig) -> Result<Self, NatsError> {
        let js =
            client.config().jetstream.clone().ok_or_else(|| {
                NatsError::configuration("NatsConsumer requires jetstream enabled")
            })?;
        let subjects = client
            .subjects()
            .expect("prefixo de subject validado no build da config");
        let mut config = config;
        if config.filter_subjects.is_empty() {
            config.filter_subjects = vec![subjects.all()];
        }
        let _ = js; // configuração jetstream necessária apenas como gate
        Ok(Self {
            client: client.clone(),
            config,
            subjects,
        })
    }

    /// Configuração do consumidor.
    pub fn config(&self) -> &NatsConsumerConfig {
        &self.config
    }

    /// Builder de subjects.
    pub fn subjects(&self) -> &SubjectBuilder {
        &self.subjects
    }

    fn context(&self) -> Result<async_nats::jetstream::Context, NatsError> {
        let js =
            self.client.config().jetstream.as_ref().ok_or_else(|| {
                NatsError::configuration("NatsConsumer requires jetstream enabled")
            })?;
        let raw = self.client.connection().clone();
        Ok(match &js.domain {
            Some(domain) => async_nats::jetstream::with_domain(raw, domain.clone()),
            None => async_nats::jetstream::new(raw),
        })
    }

    /// Garante o consumer durável no servidor (idempotente).
    ///
    /// `ack_policy = Explicit`, `ack_wait` e `max_deliver` conforme a
    /// configuração; a criação não destrói um consumer existente com
    /// a mesma definição.
    pub async fn ensure_consumer(&self) -> Result<(), NatsError> {
        let consumer = self.pull_consumer().await?;
        info!(
            stream = %self.config.stream,
            durable = %self.config.durable_name,
            "consumer duravel garantido"
        );
        drop(consumer);
        Ok(())
    }

    async fn pull_consumer(
        &self,
    ) -> Result<
        async_nats::jetstream::consumer::Consumer<async_nats::jetstream::consumer::pull::Config>,
        NatsError,
    > {
        use async_nats::jetstream::consumer::{self, AckPolicy, DeliverPolicy};

        let context = self.context()?;
        let stream = context
            .get_stream(self.config.stream.clone())
            .await
            .map_err(|source| {
                NatsError::JetStream(format!(
                    "get stream `{}` failed: {source}",
                    self.config.stream
                ))
            })?;

        let deliver_policy = if self.config.replay_from_start {
            DeliverPolicy::All
        } else {
            DeliverPolicy::New
        };

        stream
            .get_or_create_consumer(
                self.config.durable_name.as_str(),
                consumer::pull::Config {
                    durable_name: Some(self.config.durable_name.clone()),
                    ack_policy: AckPolicy::Explicit,
                    ack_wait: self.config.ack_wait,
                    max_deliver: self.config.max_deliver as i64,
                    filter_subjects: self.config.filter_subjects.clone(),
                    deliver_policy,
                    ..Default::default()
                },
            )
            .await
            .map_err(|source| {
                NatsError::JetStream(format!(
                    "ensure consumer `{}` failed: {source}",
                    self.config.durable_name
                ))
            })
    }

    /// Consome indefinidamente, despachando cada mensagem ao `handler`.
    ///
    /// O `handler` devolve `Ok(())` (double-ACK) ou `Err` (no-ACK →
    /// redelivery do JetStream após `ack_wait`). Erros do handler são
    /// **registrados e engolidos** — a semântica de retry é do broker.
    pub async fn consume<F, Fut>(&self, handler: F) -> Result<(), NatsError>
    where
        F: Fn(NatsDelivered) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), NatsError>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let consumer = self.pull_consumer().await?;
        info!(
            stream = %self.config.stream,
            durable = %self.config.durable_name,
            "consumindo jetstream"
        );

        loop {
            let mut messages = consumer
                .fetch()
                .max_messages(self.config.batch_size)
                .messages()
                .await
                .map_err(|source| NatsError::JetStream(format!("fetch failed: {source}")))?;

            while let Some(result) = messages.next().await {
                let message = result.map_err(|source| {
                    NatsError::JetStream(format!("fetch batch item failed: {source}"))
                })?;
                self.process_message(&message, &handler).await?;
            }
        }
    }

    /// Consome eventos **tipados** com um [`EventHandler`] do `gerax-eda`.
    ///
    /// Cada mensagem vira um [`EventEnvelope<E>`] (id, metadata de
    /// correlação/causação preservados) entregue ao handler. Falha do
    /// handler → no-ACK → redelivery (at-least-once; idempotência é do
    /// consumidor — `§15`).
    pub async fn consume_event<E, H>(&self, handler: H) -> Result<(), NatsError>
    where
        E: Event + DeserializeOwned,
        H: EventHandler<E> + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        self.consume(move |delivered: NatsDelivered| {
            let handler = Arc::clone(&handler);
            async move {
                let event = delivered.envelope.payload::<E>()?;
                let event_envelope = EventEnvelope {
                    id: delivered.envelope.event_id(),
                    event,
                    metadata: delivered.envelope.to_eda_metadata(),
                };
                handler
                    .handle(event_envelope)
                    .await
                    .map_err(|source| NatsError::Handler(source.to_string()))
            }
        })
        .await
    }

    async fn process_message<F, Fut>(
        &self,
        message: &async_nats::jetstream::Message,
        handler: &Arc<F>,
    ) -> Result<(), NatsError>
    where
        F: Fn(NatsDelivered) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), NatsError>> + Send + 'static,
    {
        let info = message
            .info()
            .map_err(|source| NatsError::JetStream(format!("message info failed: {source}")))?;
        let delivered = info.delivered.max(0) as u64;
        let subject = message.subject.to_string();

        let envelope = match envelope::decode_envelope(&message.payload) {
            Ok(envelope) => envelope,
            Err(source) => {
                warn!(subject = %subject, delivered, error = %source, "payload indecifrável → DLQ");
                self.forward_dlq_raw(&subject, delivered, &source.to_string(), &message.payload)
                    .await?;
                self.ack(message).await?;
                return Ok(());
            }
        };

        if delivered > self.config.max_deliver {
            warn!(
                subject = %subject,
                delivered,
                max_deliver = self.config.max_deliver,
                "entregas excedidas → DLQ"
            );
            self.forward_dlq(&envelope, &subject, delivered, "max_deliver exceeded")
                .await?;
            self.ack(message).await?;
            return Ok(());
        }

        let delivered_msg = NatsDelivered {
            envelope: envelope.clone(),
            subject: subject.clone(),
            stream: info.stream.to_string(),
            consumer: info.consumer.to_string(),
            delivered,
            stream_sequence: info.stream_sequence,
        };

        let started = Instant::now();
        match handler(delivered_msg).await {
            Ok(()) => {
                self.double_ack(message).await?;
                debug!(
                    subject = %subject,
                    delivered,
                    duration_ms = started.elapsed().as_millis() as u64,
                    status = "success",
                    "mensagem processada e confirmada"
                );
                Ok(())
            }
            Err(source) => {
                if delivered >= self.config.max_deliver {
                    warn!(
                        subject = %subject,
                        delivered,
                        error = %source,
                        "ultima entrega falhou → DLQ"
                    );
                    self.forward_dlq(&envelope, &subject, delivered, &source.to_string())
                        .await?;
                    self.ack(message).await?;
                } else {
                    warn!(
                        subject = %subject,
                        delivered,
                        error = %source,
                        remaining = self.config.max_deliver - delivered,
                        duration_ms = started.elapsed().as_millis() as u64,
                        status = "no-ack (redelivery pelo JetStream)"
                    );
                }
                Ok(())
            }
        }
    }

    async fn ack(&self, message: &async_nats::jetstream::Message) -> Result<(), NatsError> {
        message
            .ack()
            .await
            .map_err(|source| NatsError::JetStream(format!("ack failed: {source}")))
    }

    async fn double_ack(&self, message: &async_nats::jetstream::Message) -> Result<(), NatsError> {
        message
            .double_ack()
            .await
            .map_err(|source| NatsError::JetStream(format!("double-ack failed: {source}")))
    }

    async fn forward_dlq(
        &self,
        envelope: &NatsEnvelope,
        subject: &str,
        delivered: u64,
        error: &str,
    ) -> Result<(), NatsError> {
        let dlq_subject = match &self.config.dlq {
            Some(dlq) => self.subjects.dlq(&dlq.service, &envelope.event_type)?,
            None => {
                error!(
                    subject = %subject,
                    event_type = %envelope.event_type,
                    delivered,
                    error,
                    "mensagem poison descartada sem DLQ configurada (ative dead_letter())"
                );
                return Ok(());
            }
        };

        let message = DlqMessage {
            event_id: envelope.event_id.clone(),
            event_type: envelope.event_type.clone(),
            subject: subject.to_string(),
            delivered,
            error: error.to_string(),
            failed_at: chrono::Utc::now(),
            correlation_id: envelope.correlation_id.clone(),
            causation_id: envelope.causation_id.clone(),
            payload: envelope.payload.clone(),
        };

        self.publish_dlq(&dlq_subject, message).await
    }

    async fn forward_dlq_raw(
        &self,
        subject: &str,
        delivered: u64,
        error: &str,
        raw_payload: &[u8],
    ) -> Result<(), NatsError> {
        let dlq_subject = match &self.config.dlq {
            Some(dlq) => self.subjects.dlq(&dlq.service, "unknown")?,
            None => {
                error!(
                    subject = %subject,
                    delivered,
                    error,
                    "payload indecifrável descartado sem DLQ configurada (ative dead_letter())"
                );
                return Ok(());
            }
        };

        let payload = serde_json::from_slice::<Value>(raw_payload).unwrap_or(Value::Null);
        let message = DlqMessage {
            event_id: None,
            event_type: "unknown".into(),
            subject: subject.to_string(),
            delivered,
            error: error.to_string(),
            failed_at: chrono::Utc::now(),
            correlation_id: None,
            causation_id: None,
            payload,
        };

        self.publish_dlq(&dlq_subject, message).await
    }

    async fn publish_dlq(&self, dlq_subject: &str, message: DlqMessage) -> Result<(), NatsError> {
        let context = self.context()?;
        let payload = bytes::Bytes::from(serde_json::to_vec(&message)?);
        let ack = context
            .publish(dlq_subject.to_string(), payload)
            .await
            .map_err(|source| NatsError::Publish(format!("dlq publish failed: {source}")))?
            .await
            .map_err(|source| NatsError::Publish(format!("dlq publish failed: {source}")))?;
        warn!(
            subject = %dlq_subject,
            stream = %ack.stream,
            sequence = ack.sequence,
            "mensagem encaminhada para DLQ"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_sao_conservadores() {
        let config = NatsConsumerConfig::new("treatment-service");
        assert_eq!(config.stream, "GERAX_EVENTS");
        assert_eq!(config.ack_wait, DEFAULT_ACK_WAIT);
        assert_eq!(config.max_deliver, 5);
        assert!(config.replay_from_start);
        assert!(config.dlq.is_none());
    }

    #[test]
    fn builder_rejeita_durable_vazio() {
        let err = NatsConsumerConfig::builder()
            .durable_name("")
            .build()
            .unwrap_err();
        assert!(matches!(err, NatsError::Configuration(_)));
    }

    #[test]
    fn builder_normaliza_max_deliver_e_batch() {
        let config = NatsConsumerConfig::builder()
            .durable_name("svc")
            .max_deliver(0)
            .batch_size(0)
            .dead_letter("svc")
            .build()
            .unwrap();
        assert_eq!(config.max_deliver, 1);
        assert_eq!(config.batch_size, 1);
        assert_eq!(config.dlq.unwrap().service, "svc");
    }

    #[test]
    fn dlq_message_serializa_estavelmente() {
        let message = DlqMessage {
            event_id: Some("01J".into()),
            event_type: "aluno.matriculado".into(),
            subject: "gerax.aluno.matriculado".into(),
            delivered: 5,
            error: "boom".into(),
            failed_at: chrono::Utc::now(),
            correlation_id: Some("fluxo-1".into()),
            causation_id: None,
            payload: serde_json::json!({ "aluno_id": "a-1" }),
        };

        let json = serde_json::to_value(&message).unwrap();
        assert_eq!(json["event_type"], "aluno.matriculado");
        assert_eq!(json["delivered"], 5);
        assert_eq!(json["payload"]["aluno_id"], "a-1");
    }
}
