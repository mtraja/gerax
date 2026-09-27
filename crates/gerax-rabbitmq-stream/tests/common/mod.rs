//! Helpers compartilhados pelos testes de integração (§33).
//!
//! Os testes exigem um RabbitMQ **real com Stream habilitado**
//! (porta 5552). Sem as variáveis de ambiente, cada teste é ignorado
//! com uma mensagem `skipped` — nenhum mock do protocolo Streams é
//! usado (§33).
//!
//! | Variável | Padrão | Uso |
//! |---|---|---|
//! | `RABBITMQ_STREAM_HOST` | — | gate: sem ela os testes são pulados |
//! | `RABBITMQ_STREAM_PORT` | `5552` | porta do plugin de streams |
//! | `RABBITMQ_STREAM_USER` | `guest` | usuário |
//! | `RABBITMQ_STREAM_PASSWORD` | `guest` | senha |
//! | `RABBITMQ_STREAM_VHOST` | `/` | virtual host |

// Cada target de teste compila este módulo apenas com o que usa.
#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
use gerax_rabbitmq_stream::{
    ConsumerConfig, InMemoryOffsetStore, JsonEventSerializer, OffsetStrategy, PublishMode,
    RabbitMqStreamConfig, RabbitMqStreamConnection, StartOffset, StreamConfig,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use rabbitmq_stream_client::types::Message;

/// Instala o `tracing` dos testes (respeita `RUST_LOG`).
///
/// Sem isso, os logs de §30 (`consumer.message_received`,
/// `producer.confirmed`, ...) não aparecem nem com `--nocapture`.
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();
}

/// Gate dos testes: devolve a config de conexão ou `None` (teste pulado).
pub fn gate_broker() -> Option<RabbitMqStreamConfig> {
    init_tracing();

    let host = std::env::var("RABBITMQ_STREAM_HOST")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let Some(host) = host else {
        eprintln!(
            "skipped: set RABBITMQ_STREAM_HOST (and RABBITMQ_STREAM_PORT) to run \
             RabbitMQ Streams integration tests"
        );
        return None;
    };

    let port = std::env::var("RABBITMQ_STREAM_PORT")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(5552);

    let mut builder = RabbitMqStreamConfig::builder()
        .host(host)
        .port(port)
        .connection_name("gerax-rabbitmq-stream-test");

    if let Ok(user) = std::env::var("RABBITMQ_STREAM_USER") {
        builder = builder.username(user);
    }
    if let Ok(password) = std::env::var("RABBITMQ_STREAM_PASSWORD") {
        builder = builder.password(password);
    }
    if let Ok(vhost) = std::env::var("RABBITMQ_STREAM_VHOST") {
        builder = builder.virtual_host(vhost);
    }

    Some(builder.build().expect("config de teste valida"))
}

/// Nome de stream/consumer único por teste (evita colisão entre
/// execuções paralelas).
pub fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}

/// Conexão + stream já criado, com nome exclusivo.
pub struct TestBroker {
    pub connection: RabbitMqStreamConnection,
    pub stream: String,
}

impl TestBroker {
    pub async fn connect(config: RabbitMqStreamConfig, prefix: &str) -> Self {
        let stream = unique(prefix);
        let connection = RabbitMqStreamConnection::connect(config, StreamConfig::new(&stream))
            .await
            .expect("conexão com o broker de streams");
        connection
            .ensure_stream()
            .await
            .expect("stream criado para o teste");
        Self { connection, stream }
    }

    pub async fn open(prefix: &str) -> Option<Self> {
        let config = gate_broker()?;
        Some(Self::connect(config, prefix).await)
    }

    /// Consumer de teste começando do início do stream.
    ///
    /// `StartOffset::First` é o padrão dos testes porque muitos
    /// publicam **antes** de assinar; com o `Next` padrão do adapter
    /// nada seria lido.
    pub fn consumer_config(&self, consumer_name: &str) -> ConsumerConfig {
        ConsumerConfig::builder()
            .stream(&self.stream)
            .consumer_name(consumer_name)
            .offset(StartOffset::First)
            .build()
            .expect("config de consumidor valida")
    }

    /// Producer confirmado para o stream do teste.
    pub async fn producer(&self) -> gerax_rabbitmq_stream::RabbitMqStreamProducer {
        self.connection
            .producer(PublishMode::Confirmed, 100, JsonEventSerializer)
            .await
            .expect("producer de teste")
    }

    /// Publica uma mensagem **crua** (sem envelope), para exercitar o
    /// caminho de payload malformado.
    pub async fn publish_raw(&self, body: &[u8]) {
        let producer = self
            .connection
            .environment()
            .producer()
            .client_provided_name("gerax-rabbitmq-stream-test-raw")
            .build(&self.stream)
            .await
            .expect("producer raw");
        let message = Message::builder().body(body.to_vec()).build();
        producer
            .send_with_confirm(message)
            .await
            .expect("publicação raw confirmada");
        producer.close().await.expect("producer raw fechado");
    }

    /// Remove o stream de teste (higiene do broker entre execuções).
    pub async fn cleanup(&self) {
        let _ = self.connection.delete_stream().await;
    }
}

// ─────────────────────────── eventos de teste ───────────────────────────

/// Evento de teste que circula pelo stream.
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

/// Evento sem handler registrado — exercita o caminho "evento de outro
/// contexto".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurmaCriada {
    pub turma_id: String,
}

impl Event for TurmaCriada {
    fn event_type(&self) -> &'static str {
        "turma.criada"
    }
}

/// Coleta os envelopes recebidos em um canal para a asserção do teste.
#[derive(Clone)]
pub struct CollectHandler {
    pub tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
}

impl CollectHandler {
    pub fn new(tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>) -> Self {
        Self { tx }
    }
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

/// Handler que falha nas primeiras `failures` chamadas e depois
/// entrega o envelope no canal.
pub struct FailThenCollectHandler {
    pub failures: AtomicUsize,
    pub seen: AtomicUsize,
    pub tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
}

impl FailThenCollectHandler {
    pub fn new(
        failures: usize,
    ) -> (
        Self,
        mpsc::UnboundedReceiver<EventEnvelope<AlunoMatriculado>>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                failures: AtomicUsize::new(failures),
                seen: AtomicUsize::new(0),
                tx,
            },
            rx,
        )
    }
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for FailThenCollectHandler {
    async fn handle(
        &self,
        event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        self.seen.fetch_add(1, Ordering::SeqCst);
        if self.failures.load(Ordering::SeqCst) > 0 {
            self.failures.fetch_sub(1, Ordering::SeqCst);
            return Err(EventHandlerError::Retryable("falha de teste".into()));
        }
        self.tx
            .send(event)
            .map_err(|_| EventHandlerError::NonRetryable("collector channel closed".into()))
    }
}

/// Handler que sempre falha (retryable) — o offset não pode avançar.
///
/// O contador é compartilhado por `Arc` para que o teste possa observar
/// as chamadas sem precisar do handler em si.
pub struct AlwaysFailHandler {
    pub calls: Arc<AtomicUsize>,
}

impl AlwaysFailHandler {
    pub fn new() -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                calls: Arc::clone(&calls),
            },
            calls,
        )
    }
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for AlwaysFailHandler {
    async fn handle(
        &self,
        _event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(EventHandlerError::Retryable("sempre falha".into()))
    }
}

/// Store client-side compartilhado entre duas subscriptions (simula
/// restart do consumidor).
pub fn client_side_strategy() -> (OffsetStrategy, Arc<InMemoryOffsetStore>) {
    let store = Arc::new(InMemoryOffsetStore::new());
    (strategy_for(&store), store)
}

/// Estratégia client-side sobre um store já existente.
pub fn strategy_for(store: &Arc<InMemoryOffsetStore>) -> OffsetStrategy {
    OffsetStrategy::client_side(Arc::clone(store) as Arc<dyn gerax_rabbitmq_stream::OffsetStore>)
}

/// Envelope no formato do adapter, a partir de um evento EDA.
pub fn envelope<E>(event: E) -> gerax_rabbitmq_stream::MessageEnvelope
where
    E: Event + Serialize,
{
    let eda = EventEnvelope::new(event);
    gerax_rabbitmq_stream::MessageEnvelope::from_eda(&eda, &JsonEventSerializer)
        .expect("envelope de teste")
}

/// Aguarda uma entrega, sem bloquear além do timeout.
pub async fn next_with_timeout<E>(
    rx: &mut mpsc::UnboundedReceiver<EventEnvelope<E>>,
    timeout: Duration,
) -> EventEnvelope<E> {
    tokio::time::timeout(timeout, rx.recv())
        .await
        .expect("entrega esperada dentro do timeout")
        .expect("canal encerrado antes da entrega")
}

/// Afirma que **nada** chega no canal dentro de `window`.
pub async fn expect_no_delivery<E>(
    rx: &mut mpsc::UnboundedReceiver<EventEnvelope<E>>,
    window: Duration,
) where
    E: std::fmt::Debug,
{
    if let Ok(delivery) = tokio::time::timeout(window, rx.recv()).await {
        panic!("entrega inesperada: {delivery:?}");
    }
}

/// Evento de teste com posição previsível.
pub fn matriculado(aluno_id: &str) -> AlunoMatriculado {
    AlunoMatriculado {
        aluno_id: aluno_id.to_string(),
        curso: "CMF".to_string(),
    }
}
