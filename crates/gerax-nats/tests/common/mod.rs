//! Helpers compartilhados pelos testes de integração do `gerax-nats`.
//!
//! Os testes exigem um servidor NATS real (`nats-server -js` para os
//! testes JetStream). Sem a variável `NATS_URL`, todos os testes são
//! ignorados com a mensagem `skipped` (padrão do `gerax-rabbitmq`).

// Cada target de teste compila este módulo apenas com o que usa.
#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
use gerax_nats::{JetStreamConfig, NatsClient, NatsConfig, NatsJetStream, StreamStorage};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Timeout de entrega dos testes (redelivery cabe dentro dele).
pub const DELIVERY_TIMEOUT: Duration = Duration::from_secs(10);

/// URL do servidor lida de `NATS_URL`.
pub fn nats_url() -> Option<String> {
    std::env::var("NATS_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Interrompe o teste quando o servidor não está configurado.
pub fn gate_nats() -> Option<String> {
    let Some(url) = nats_url() else {
        eprintln!("skipped: set NATS_URL to run integration tests");
        return None;
    };
    Some(url)
}

/// Namespace de teste: prefixo de subjects e stream **únicos** por
/// teste (streams JetStream paralelas não podem capturar o mesmo
/// subject).
pub struct TestNamespace {
    /// Prefixo dos subjects (`gt.<teste>-<uuid>`).
    pub prefix: String,
    /// Nome da stream JetStream (`GERAX_TEST_<uuid>`).
    pub stream: String,
}

/// Cria um namespace único para um teste.
pub fn namespace(test: &str) -> TestNamespace {
    TestNamespace {
        prefix: format!("gt.{test}-{}", uuid::Uuid::new_v4()),
        stream: format!("GERAX_TEST_{}", uuid::Uuid::new_v4().simple()),
    }
}

/// Configuração Core NATS (sem JetStream) para o namespace.
pub fn core_config(url: &str, ns: &TestNamespace) -> NatsConfig {
    NatsConfig::builder()
        .url(url)
        .name(format!("gerax-nats-test-{}", ns.prefix))
        .subject_prefix(&ns.prefix)
        .jetstream_disabled()
        .build()
        .expect("config core de teste valida")
}

/// Configuração JetStream (stream de memória) para o namespace.
pub fn js_config(url: &str, ns: &TestNamespace) -> NatsConfig {
    NatsConfig::builder()
        .url(url)
        .name(format!("gerax-nats-test-{}", ns.prefix))
        .subject_prefix(&ns.prefix)
        .jetstream(
            JetStreamConfig::new()
                .stream(&ns.stream)
                .storage(StreamStorage::Memory),
        )
        .build()
        .expect("config jetstream de teste valida")
}

/// Conecta ao servidor com a configuração fornecida.
pub async fn connect(url: &str, ns: &TestNamespace, jetstream: bool) -> NatsClient {
    let config = if jetstream {
        js_config(url, ns)
    } else {
        core_config(url, ns)
    };
    NatsClient::connect(config).await.expect("conexao nats")
}

/// Garante a stream JetStream do namespace (criação idempotente).
pub async fn ensure_stream(client: &NatsClient, ns: &TestNamespace) -> NatsJetStream {
    let jetstream = NatsJetStream::new(
        client,
        JetStreamConfig::new()
            .stream(&ns.stream)
            .storage(StreamStorage::Memory),
    );
    jetstream.ensure_stream().await.expect("stream garantida");
    jetstream
}

/// Apaga a stream do namespace no fim do teste (melhor esforço).
pub async fn cleanup_stream(client: &NatsClient, ns: &TestNamespace) {
    let jetstream = NatsJetStream::new(
        client,
        JetStreamConfig::new()
            .stream(&ns.stream)
            .storage(StreamStorage::Memory),
    );
    let _ = jetstream.delete_stream().await;
}

/// Nome de consumer durável único e válido (sem pontos — o JetStream
/// rejeita `.` em nomes de consumer).
pub fn durable(test: &str) -> String {
    format!("{test}-{}", uuid::Uuid::new_v4().simple())
}

/// Assina um subject exato e devolve o subscriber pronto.
pub async fn next_subscription(client: &NatsClient, subject: &str) -> async_nats::Subscriber {
    client
        .subscribe(subject)
        .await
        .expect("subscription criada")
}

/// Aguarda a próxima mensagem de um subscriber (com timeout).
pub async fn next_message(
    subscriber: &mut async_nats::Subscriber,
    timeout: Duration,
) -> async_nats::Message {
    tokio::time::timeout(timeout, subscriber.next())
        .await
        .expect("mensagem dentro do timeout")
        .expect("subscription aberta")
}

// ─────────────────────────── eventos de teste ───────────────────────────

/// Evento de teste que circula pelo broker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlunoMatriculado {
    pub aluno_id: String,
    pub curso: String,
}

impl Event for AlunoMatriculado {
    fn event_type(&self) -> &'static str {
        "aluno.matriculado"
    }
}

/// Envelope EDA de teste com correlação e causação.
pub fn envelope_eda(aluno_id: &str) -> EventEnvelope<AlunoMatriculado> {
    EventEnvelope::new(AlunoMatriculado {
        aluno_id: aluno_id.into(),
        curso: "matematica".into(),
    })
    .with_correlation_id("fluxo-teste")
    .with_causation_id("evento-anterior")
}

/// Aguarda o próximo envelope no canal do collector.
pub async fn next_envelope(
    rx: &mut mpsc::UnboundedReceiver<EventEnvelope<AlunoMatriculado>>,
) -> EventEnvelope<AlunoMatriculado> {
    tokio::time::timeout(DELIVERY_TIMEOUT, rx.recv())
        .await
        .expect("entrega dentro do timeout")
        .expect("canal do collector aberto")
}

/// Aguarda a próxima entrega bruta no canal do collector.
pub async fn next_delivery(
    rx: &mut mpsc::UnboundedReceiver<gerax_nats::NatsDelivered>,
) -> gerax_nats::NatsDelivered {
    tokio::time::timeout(DELIVERY_TIMEOUT, rx.recv())
        .await
        .expect("entrega dentro do timeout")
        .expect("canal do collector aberto")
}

/// Handler que apenas registra os envelopes no canal.
#[derive(Clone)]
pub struct CollectHandler {
    pub tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for CollectHandler {
    async fn handle(
        &self,
        event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        self.tx
            .send(event)
            .map_err(|_| EventHandlerError::NonRetryable("collector channel closed".into()))
    }
}

/// Handler que falha na primeira tentativa e registra as demais.
pub struct FailOnceHandler {
    pub attempts: Arc<AtomicUsize>,
    pub tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for FailOnceHandler {
    async fn handle(
        &self,
        event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(EventHandlerError::Retryable(
                "primeira tentativa falha".into(),
            ));
        }
        self.tx
            .send(event)
            .map_err(|_| EventHandlerError::NonRetryable("collector channel closed".into()))
    }
}

/// Handler que sempre falha (→ DLQ após `max_deliver`).
pub struct AlwaysFailHandler;

#[async_trait]
impl EventHandler<AlunoMatriculado> for AlwaysFailHandler {
    async fn handle(
        &self,
        _event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        Err(EventHandlerError::NonRetryable(
            "falha deterministica de teste".into(),
        ))
    }
}
