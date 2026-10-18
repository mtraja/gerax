//! Exemplo completo: publish-subscribe sobre **RabbitMQ Streams**.
//!
//! Roda contra o broker de desenvolvimento com o plugin de streams
//! habilitado (`compose.stream.yaml` na raiz do workspace,
//! `podman-compose -f compose.stream.yaml up -d`):
//!
//! ```console
//! cargo run -p gerax-rabbitmq-stream --example aluno_matriculado
//! RABBITMQ_STREAM_HOST=broker RABBITMQ_STREAM_PORT=5552 \
//!     cargo run -p gerax-rabbitmq-stream --example aluno_matriculado
//! ```
//!
//! Demonstra:
//!
//! - `RabbitMqStreamEventBus` publicando `aluno.matriculado` no stream
//!   `school-events` (criado no bootstrap, se ainda não existir);
//! - envelope em duas camadas: payload no `body`, rastreabilidade em
//!   `application_properties` (`event_id`, `occurred_at`,
//!   `correlation_id`, ...);
//! - `RabbitMqStreamConsumer` roteando por `event_type` e delivering o
//!   evento **tipado** ao handler do `gerax-eda`;
//! - at-least-once: o offset é gravado **depois** do handler, então o
//!   evento volta se o processo cair no meio;
//! - shutdown ordenado: a subscription para antes do producer.

use std::time::Duration;

use async_trait::async_trait;
use gerax_eda::{Event, EventEnvelope, EventBus, EventHandler, EventHandlerError};
use gerax_rabbitmq_stream::{
    ConsumerConfig, PublishMode, RabbitMqStreamConfig, RabbitMqStreamEventBusBuilder, StartOffset,
    StreamConfig,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Evento de domínio (declarado no `gerax-eda`).
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
struct ApresentarAluno {
    tx: mpsc::UnboundedSender<EventEnvelope<AlunoMatriculado>>,
}

#[async_trait]
impl EventHandler<AlunoMatriculado> for ApresentarAluno {
    async fn handle(
        &self,
        envelope: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        println!(
            "  [handler] {} → aluno {} matriculado em {} (event_id {}, correlação {})",
            envelope.event_type(),
            envelope.event.aluno_id,
            envelope.event.curso,
            envelope.id.as_uuid(),
            envelope
                .metadata
                .correlation_id
                .as_ref()
                .map(|id| id.as_str())
                .unwrap_or("-"),
        );
        self.tx
            .send(envelope)
            .map_err(|_| EventHandlerError::NonRetryable("channel fechado".into()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gerax_rabbitmq_stream=info".into()),
        )
        .init();

    let host = std::env::var("RABBITMQ_STREAM_HOST").unwrap_or_else(|_| "localhost".into());
    let port = std::env::var("RABBITMQ_STREAM_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(5552);
    let stream_name =
        std::env::var("RABBITMQ_STREAM_NAME").unwrap_or_else(|_| "school-events".to_string());

    let config = RabbitMqStreamConfig::builder()
        .host(host)
        .port(port) // protocolo de streams, não 5672 (AMQP)
        .connection_name("gerax-rabbitmq-stream-example")
        .build()?;

    // ── bus (cria o stream se necessário) ────────────────────────
    let bus = RabbitMqStreamEventBusBuilder::new(config, StreamConfig::new(&stream_name))
        .publish_mode(PublishMode::Confirmed)
        .build()
        .await?;

    // ── consumidor ──────────────────────────────────────────────
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = bus
        .connection()
        .consumer(
            ConsumerConfig::builder()
                .stream(&stream_name)
                .consumer_name("school-consumer")
                .offset(StartOffset::First)
                .build()?,
            gerax_rabbitmq_stream::JsonEventSerializer,
        )
        .await?;

    consumer.register::<AlunoMatriculado, _>("aluno.matriculado", ApresentarAluno { tx });

    let subscription = consumer.subscribe().await?;
    println!("assinando o stream `{stream_name}`...");

    // publica três eventos; o consumidor os entrega ao handler
    println!("publicando 3 eventos...");
    for i in 1..=3 {
        bus.publish(
            EventEnvelope::new(AlunoMatriculado {
                aluno_id: format!("a-{i}"),
                curso: "matematica".into(),
            })
            .with_correlation_id(format!("matricula-{i}")),
        )
        .await?;
        println!("  [bus] publicado aluno.matriculado a-{i} (confirmado pelo broker)");
    }

    println!("aguardando processamento...");
    for _ in 0..3 {
        // Timeout generoso: o stream entrega em lote e o handler pode
        // demorar em uma projeção real.
        let _ = tokio::time::timeout(Duration::from_secs(10), rx.recv())
            .await
            .expect("handler respondeu dentro do timeout")
            .expect("canal do handler aberto");
    }
    println!("ok: publish-subscribe em RabbitMQ Streams concluído");

    // Shutdown: para a subscription e depois fecha o producer.
    subscription.stop().await?;
    bus.shutdown().await?;

    Ok(())
}
