//! Helpers compartilhados pelos testes de integração.
//!
//! Os testes exigem um RabbitMQ real. Quando a variável de ambiente
//! `RABBITMQ_URI` não está definida, os testes são ignorados com uma
//! mensagem `skipped` (veja `gate_rabbitmq` e `connection_if_available`).

// Cada target de teste compila este módulo apenas com o que usa.
#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
use gerax_rabbitmq::{
    RabbitConsumerConfig, RabbitHandlerRegistry, RabbitMqConfig, RabbitMqConnection,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// URI do broker lida de `RABBITMQ_URI` (sem shebang de credenciais em
/// logs — aponta para um broker local de desenvolvimento).
pub fn rabbitmq_uri() -> Option<String> {
    std::env::var("RABBITMQ_URI")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Interrompe o teste quando o broker não está configurado.
pub fn gate_rabbitmq() -> Option<String> {
    let Some(uri) = rabbitmq_uri() else {
        eprintln!("skipped: set RABBITMQ_URI to run integration tests");
        return None;
    };
    Some(uri)
}

/// Configuração de conexão com exchange/fila **únicos** por teste.
pub fn config(uri: &str, name: &str) -> RabbitMqConfig {
    RabbitMqConfig::builder()
        .uri(uri)
        .exchange(format!("test.{name}.exchange"))
        .connection_name(format!("gerax-rabbitmq-test-{name}"))
        .build()
        .expect("config de teste valida")
}

/// Nome único por teste (evita colisão entre testes paralelos).
pub fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}

/// Abre a conexão de teste somente quando o broker está disponível
/// (gated por `RABBITMQ_URI`).
pub async fn connection_if_available() -> Option<RabbitMqConnection> {
    let uri = gate_rabbitmq()?;
    Some(
        RabbitMqConnection::connect(config(&uri, "gate"))
            .await
            .expect("conexão de teste com o broker"),
    )
}

/// Cria o bus de eventos sobre a conexão de teste.
pub async fn bus_for(
    connection: &RabbitMqConnection,
    config: &RabbitMqConfig,
) -> gerax_rabbitmq::RabbitMqEventBus {
    gerax_rabbitmq::RabbitMqEventBus::from_connection(connection, config.clone())
        .await
        .expect("bus")
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

/// Coleta os envelopes recebidos em um canal para a asserção do teste.
#[derive(Clone)]
pub struct CollectHandler<E> {
    pub tx: mpsc::UnboundedSender<EventEnvelope<E>>,
}

impl<E> CollectHandler<E> {
    pub fn new(tx: mpsc::UnboundedSender<EventEnvelope<E>>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for CollectHandler<AlunoMatriculado> {
    async fn handle(
        &self,
        event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        if self.tx.send(event).is_err() {
            return Err(EventHandlerError::NonRetryable(
                "collector channel closed".into(),
            ));
        }
        Ok(())
    }
}

/// Handler que falha com [`EventHandlerError::Retryable`] nas primeiras
/// `failures` chamadas e depois registra os envelopes no canal.
pub struct RetryThenCollectHandler {
    pub failures: AtomicUsize,
    pub tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for RetryThenCollectHandler {
    async fn handle(
        &self,
        event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        if self.failures.load(Ordering::SeqCst) > 0 {
            self.failures.fetch_sub(1, Ordering::SeqCst);
            return Err(EventHandlerError::Retryable(
                "primeira tentativa falha".into(),
            ));
        }
        self.tx
            .send(event)
            .map_err(|_| EventHandlerError::NonRetryable("collector channel closed".into()))
    }
}

/// Handler que sempre falha de forma **não-retryável** (→ DLQ).
pub struct AlwaysFailHandler;

#[async_trait]
impl EventHandler<AlunoMatriculado> for AlwaysFailHandler {
    async fn handle(
        &self,
        _event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        Err(EventHandlerError::NonRetryable(
            "falha determinística de teste".into(),
        ))
    }
}

/// Configuração de consumidor de teste com retry rápido.
pub fn consumer_config(queue: &str, name: &str) -> RabbitConsumerConfig {
    let _ = name; // tag de conexão é definida na `RabbitMqConfig` do teste
    RabbitConsumerConfig::builder()
        .queue(queue)
        .retry(gerax_rabbitmq::RetryPolicy::new(
            2,
            Duration::from_millis(150),
            1.0,
        ))
        .build()
        .expect("config de consumidor valida")
}

/// Registra o handler de coleta no registry para `aluno.matriculado`.
pub fn register_collector(
    registry: &RabbitHandlerRegistry,
    tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
) {
    registry.register::<AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));
}

/// Aguarda até `deadline+timeout` por uma entrega no canal, sem nunca
/// bloquear o teste além do limite.
pub async fn next_with_timeout<E>(
    rx: &mut mpsc::UnboundedReceiver<EventEnvelope<E>>,
    timeout: Duration,
) -> EventEnvelope<E> {
    tokio::time::timeout(timeout, rx.recv())
        .await
        .expect("entrega esperada dentro do timeout")
        .expect("canal encerrado antes da entrega")
}

/// Converte um [`Arc`] para o consumidor em uma tarefa assíncrona.
pub fn spawn_consume(
    consumer: Arc<gerax_rabbitmq::RabbitMqConsumer>,
) -> tokio::task::JoinHandle<Result<(), gerax_rabbitmq::RabbitMqError>> {
    tokio::spawn(async move { consumer.consume().await })
}
