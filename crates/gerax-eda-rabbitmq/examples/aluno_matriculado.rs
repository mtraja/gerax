//! Publica e consome um evento `AlunoMatriculado` via RabbitMQ.
//!
//! Requer um broker RabbitMQ local (`amqp://guest:guest@localhost:5672`);
//! para sobrepor o endereço, exporte `AMQP_ADDR`... este exemplo usa a
//! configuração padrão ([`RabbitMqConfig::default`]).
//!
//! Execute com:
//!
//! ```sh
//! cargo run -p gerax-eda-rabbitmq --example aluno_matriculado
//! ```

use std::sync::Arc;
use std::time::Duration;

use gerax_eda::{Event, EventEnvelope, EventHandler, EventHandlerError};
use gerax_eda_rabbitmq::{
    JsonEventCodec, RabbitConsumerConfig, RabbitHandlerRegistry, RabbitMqConfig, RabbitMqConsumer,
    RabbitMqEventBus,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct AlunoMatriculado {
    aluno_id: String,
    curso: String,
}

impl Event for AlunoMatriculado {
    fn event_type(&self) -> &'static str {
        "aluno.matriculado"
    }
}

struct RegistrarDiario;

#[async_trait::async_trait]
impl EventHandler<AlunoMatriculado> for RegistrarDiario {
    async fn handle(
        &self,
        envelope: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        println!(
            "handler[{}]: {} matriculado em {} (evento {})",
            handler_name(),
            envelope.event.aluno_id,
            envelope.event.curso,
            envelope.id.as_uuid(),
        );
        Ok(())
    }
}

// trait de diagnóstico simplificado para o exemplo.
fn handler_name() -> &'static str {
    std::any::type_name::<RegistrarDiario>()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("gerax_eda_rabbitmq=debug")
        .init();

    let bus = RabbitMqEventBus::connect(RabbitMqConfig::default(), JsonEventCodec).await?;

    let registry = Arc::new(RabbitHandlerRegistry::new());
    registry.register::<AlunoMatriculado, _, _>(
        "aluno.matriculado",
        RegistrarDiario,
        JsonEventCodec,
    );

    let consumer = RabbitMqConsumer::connect(
        RabbitMqConfig::default(),
        RabbitConsumerConfig::default(),
        registry,
    )
    .await?;
    let consumer_task = tokio::spawn(async move { consumer.consume().await });

    bus.publish_new(AlunoMatriculado {
        aluno_id: "a-1".into(),
        curso: "matematica".into(),
    })
    .await?;
    println!("evento publicado; aguardando consumo...");

    tokio::time::sleep(Duration::from_secs(2)).await;
    consumer_task.abort();

    Ok(())
}
