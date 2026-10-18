//! Testes de integração: publicação (`§12`–`§14`, `§34`).
//!
//! Requer um RabbitMQ real. Skip quando `RABBITMQ_URI` não está setada.

mod common;

use gerax_eda::{EventEnvelope, EventId, EventMetadata, EventBus, EventBusError};
use gerax_rabbitmq::{
    EnvelopeCodec, JsonEnvelopeCodec, RabbitMqConfig, RabbitMqConnection, RabbitMqEventBus,
};
use lapin::options::{BasicAckOptions, BasicGetOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::FieldTable;

use common::{AlunoMatriculado, gate_rabbitmq, unique};

#[tokio::test]
async fn publish_com_confirm_entrega_mensagem_persistente() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("pub");
    let config = common::config(&uri, &name);
    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");
    let bus = RabbitMqEventBus::from_connection(&connection, config.clone())
        .await
        .expect("bus");

    let channel = connection.channel().await.expect("canal");
    let declare = QueueDeclareOptions {
        durable: true,
        ..Default::default()
    };
    let queue = channel
        .queue_declare(
            format!("{name}.queue").as_str().into(),
            declare,
            FieldTable::default(),
        )
        .await
        .expect("fila declarada")
        .name()
        .to_string();
    channel
        .queue_bind(
            queue.as_str().into(),
            config.exchange.clone().into(),
            "aluno.matriculado".into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("binding");

    let envelope = EventEnvelope::new(AlunoMatriculado {
        aluno_id: "a-1".into(),
        curso: "matematica".into(),
    })
    .with_correlation_id("fluxo-teste");
    bus.publish_envelope(envelope)
        .await
        .expect("publish confirmado");

    let delivery = channel
        .basic_get(queue.as_str().into(), BasicGetOptions::default())
        .await
        .expect("basic_get ok")
        .expect("mensagem publicada disponível na fila");

    let decoded = JsonEnvelopeCodec
        .decode(&delivery.data)
        .expect("decodifica envelope de transporte");
    assert_eq!(decoded.event_type(), "aluno.matriculado");
    assert_eq!(decoded.correlation_id.as_deref(), Some("fluxo-teste"));
    assert_eq!(decoded.payload["aluno_id"], serde_json::json!("a-1"));

    // metadata AMQP preservada (content type, delivery mode, type)
    assert_eq!(
        delivery
            .properties
            .content_type()
            .as_ref()
            .map(ToString::to_string),
        Some("application/json".into())
    );
    assert_eq!(delivery.properties.delivery_mode(), &Some(2));
    assert_eq!(
        delivery.properties.kind().as_ref().map(ToString::to_string),
        Some("aluno.matriculado".into())
    );

    delivery.ack(BasicAckOptions::default()).await.expect("ack");
    connection.close().await.ok();
}

#[tokio::test]
async fn publish_roteia_por_event_type() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("routing");
    let config = RabbitMqConfig::builder()
        .uri(&uri)
        .exchange(format!("test.{name}.exchange"))
        .build()
        .expect("config");
    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");
    let bus = RabbitMqEventBus::from_connection(&connection, config.clone())
        .await
        .expect("bus");

    let channel = connection.channel().await.expect("canal");
    let declare = QueueDeclareOptions {
        durable: true,
        ..Default::default()
    };
    let queue_a = channel
        .queue_declare(
            format!("{name}.a").as_str().into(),
            declare,
            FieldTable::default(),
        )
        .await
        .expect("fila a")
        .name()
        .to_string();
    let queue_b = channel
        .queue_declare(
            format!("{name}.b").as_str().into(),
            declare,
            FieldTable::default(),
        )
        .await
        .expect("fila b")
        .name()
        .to_string();

    // A only recebe `aluno.matriculado`; B recebe `matricula.cancelada`
    channel
        .queue_bind(
            queue_a.as_str().into(),
            config.exchange.clone().into(),
            "aluno.matriculado".into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("binding a");
    channel
        .queue_bind(
            queue_b.as_str().into(),
            config.exchange.clone().into(),
            "matricula.cancelada".into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("binding b");

    bus.publish_event(AlunoMatriculado {
        aluno_id: "a-1".into(),
        curso: "fisica".into(),
    })
    .await
    .expect("publish");

    let in_a = channel
        .basic_get(queue_a.as_str().into(), BasicGetOptions::default())
        .await
        .expect("get a");
    let in_b = channel
        .basic_get(queue_b.as_str().into(), BasicGetOptions::default())
        .await
        .expect("get b");

    assert!(
        in_a.is_some(),
        "fila A (aluno.matriculado) recebeu o evento"
    );
    assert!(in_b.is_none(), "fila B (matricula.cancelada) NÃO recebeu");

    if let Some(delivery) = in_a {
        delivery.ack(BasicAckOptions::default()).await.expect("ack");
    }
    connection.close().await.ok();
}

#[tokio::test]
async fn publica_via_porta_event_bus_e_valida_envelope() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("porta");
    let config = common::config(&uri, &name);
    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");
    let bus = RabbitMqEventBus::from_connection(&connection, config.clone())
        .await
        .expect("bus");

    // publica através da porta genérica (impl EventBus)
    async fn publicar<B: EventBus>(
        bus: &B,
        envelope: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventBusError> {
        bus.publish(envelope).await
    }

    publicar(
        &bus,
        EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-porta".into(),
            curso: "fisica".into(),
        }),
    )
    .await
    .expect("publicação via porta");

    // envelope inconsistente → InvalidEvent (validação da porta)
    let inconsistente = EventEnvelope {
        id: EventId::new(),
        event: AlunoMatriculado {
            aluno_id: "a-x".into(),
            curso: "fisica".into(),
        },
        metadata: EventMetadata::new("outro.tipo"),
    };
    let err = publicar(&bus, inconsistente)
        .await
        .expect_err("envelope inconsistente rejeitado");
    assert!(matches!(err, EventBusError::InvalidEvent(_)));

    connection.close().await.ok();
}
