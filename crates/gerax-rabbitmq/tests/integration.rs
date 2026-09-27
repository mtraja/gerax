//! Testes de integração ponta-a-ponta: publicar → consumir com
//! rastreabilidade preservada e política de eventos (`§34`).
//!
//! Requer um RabbitMQ real. Skip quando `RABBITMQ_URI` não está setada.

mod common;

use std::sync::Arc;
use std::time::Duration;

use gerax_eda::EventEnvelope;
use gerax_rabbitmq::{RabbitHandlerRegistry, RabbitMqConnection, RabbitMqConsumer};
use tokio::sync::mpsc;

use common::{AlunoMatriculado, gate_rabbitmq, next_with_timeout, spawn_consume, unique};

#[tokio::test]
async fn publish_envelope_preserva_rastreabilidade_ate_o_handler() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("e2e");
    let config = common::config(&uri, &name);
    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");

    let registry = Arc::new(RabbitHandlerRegistry::new());
    let (tx, mut rx) = mpsc::unbounded_channel();
    common::register_collector(&registry, tx);

    let consumer_config = common::consumer_config(&format!("{name}.queue"), &name);
    let consumer = Arc::new(
        RabbitMqConsumer::connect(&connection, consumer_config, registry)
            .await
            .expect("consumer"),
    );

    let bus = common::bus_for(&connection, &config).await;
    let handle = spawn_consume(consumer);

    let causality = EventEnvelope::new(AlunoMatriculado {
        aluno_id: "a-42".into(),
        curso: "filosofia".into(),
    })
    .with_correlation_id("caixa-efeito-77")
    .with_causation_id("evt-origem-1")
    .with_header("tenant", "escola-x");

    bus.publish_envelope(causality)
        .await
        .expect("publish confirmado");

    let received = next_with_timeout(&mut rx, Duration::from_secs(10)).await;
    assert_eq!(received.event.aluno_id, "a-42");
    assert_eq!(
        received.metadata.correlation_id.map(|id| id.to_string()),
        Some("caixa-efeito-77".into())
    );
    assert_eq!(
        received.metadata.causation_id.map(|id| id.to_string()),
        Some("evt-origem-1".into())
    );
    assert_eq!(
        received.metadata.headers.get("tenant").map(String::as_str),
        Some("escola-x")
    );

    handle.abort();
    connection.close().await.ok();
}
