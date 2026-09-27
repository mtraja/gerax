//! Exemplo completo: publish-subscribe com RabbitMQ.
//!
//! Roda contra o broker de desenvolvimento (`compose.yaml` na raiz do
//! workspace, `podman-compose up -d` / `docker compose up -d`):
//!
//! ```console
//! cargo run -p gerax-rabbitmq --example aluno_matriculado
//! RABBITMQ_URI=amqp://user:pass@broker:5672/%2f cargo run -p gerax-rabbitmq --example aluno_matriculado
//! ```
//!
//! Demonstra:
//!
//! - `EventBus` publicando eventos (`aluno.matriculado`) no exchange
//!   `topic` `school.exchange`, roteados pela `event_type`;
//! - `RabbitMqConsumer` com fila durável, **retry exponencial**
//!   (`3` tentativas, `1s × 2^n`) e **DLQ** habilitada;
//! - At-least-once: o handler roda somente após o ACK (a entrega é
//!   reconhecida **depois** de `handle()` retornar `Ok`).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
use gerax_rabbitmq::{
    DeadLetterConfig, RabbitConsumerConfig, RabbitHandlerRegistry, RabbitMqConfig,
    RabbitMqConnection, RabbitMqConsumer, RabbitMqEventBus, RetryPolicy, UnknownEventPolicy,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Evento de domínio (declarado em `gerax-eda`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AlunoMatriculado {
    aluno_id: String,
    curso: String,
}

impl Event for AlunoMatriculado {
    fn event_type(&self) -> &'static str {
        "aluno.matriculado"
    }
}

/// Handler que apresenta o evento recebido e sinaliza ao exemplo.
#[derive(Clone)]
struct ApresentarAluno {
    tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for ApresentarAluno {
    async fn handle(
        &self,
        envelope: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        let aluno = &envelope.event;
        println!(
            "  [handler] evento {} → aluno {} matriculado em {} (correlação {:?})",
            envelope.id.as_uuid(),
            aluno.aluno_id,
            aluno.curso,
            envelope.metadata.correlation_id,
        );
        self.tx
            .send(envelope)
            .map_err(|_| EventHandlerError::NonRetryable("channel fechado".into()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let uri = std::env::var("RABBITMQ_URI")
        .unwrap_or_else(|_| "amqp://gerax:gerax@localhost:5672/%2f".to_string());
    let exchange = "school.exchange";

    let config = RabbitMqConfig::new(&uri, exchange);
    let connection = Arc::new(RabbitMqConnection::connect(config.clone()).await?);

    // ── produtor ────────────────────────────────────────────────
    let bus = RabbitMqEventBus::from_connection(&connection, config.clone()).await?;

    // ── consumidor ──────────────────────────────────────────────
    let registry = Arc::new(RabbitHandlerRegistry::new());
    let (tx, mut rx) = mpsc::unbounded_channel();
    registry.register::<AlunoMatriculado, _>("aluno.matriculado", ApresentarAluno { tx });

    let consumer_config = RabbitConsumerConfig::builder()
        .queue("school.alunos")
        .durable(true)
        .retry(RetryPolicy::new(3, Duration::from_secs(1), 2.0))
        .dead_letter(DeadLetterConfig::new().enabled(true))
        .on_unknown_event(UnknownEventPolicy::Reject)
        .build()?;

    let consumer =
        Arc::new(RabbitMqConsumer::connect(&connection, consumer_config, registry).await?);
    tokio::spawn(async move { consumer.consume().await });

    // publica três eventos; o consumidor os entrega ao handler
    println!("publicando 3 eventos...");
    for i in 1..=3 {
        let aluno = AlunoMatriculado {
            aluno_id: format!("a-{i}"),
            curso: "matematica".into(),
        };
        bus.publish(aluno).await?;
        println!("  [bus] publicado aluno.matriculado a-{i}");
    }

    println!("aguardando processamento...");
    for _ in 0..3 {
        let _ = rx.recv().await.expect("handler respondeu");
    }
    println!("ok: publish-subscribe com ACK e rastreabilidade concluído");

    Ok(())
}
