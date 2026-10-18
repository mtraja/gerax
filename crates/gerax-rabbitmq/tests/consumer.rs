//! Testes de integração: consumo, ack, retry e DLQ (`§15`–`§19`, `§34`).
//!
//! Requer um RabbitMQ real. Skip quando `RABBITMQ_URI` não está setada.

mod common;

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use gerax_rabbitmq::{
    DeadLetterConfig, EnvelopeCodec, JsonEnvelopeCodec, RabbitConsumerConfig,
    RabbitHandlerRegistry, RabbitMqConnection, RabbitMqConsumer, RetryPolicy, UnknownEventPolicy,
};
use lapin::options::{BasicAckOptions, BasicGetOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use tokio::sync::mpsc;

use common::{
    AlunoMatriculado, AlwaysFailHandler, RetryThenCollectHandler, gate_rabbitmq, next_with_timeout,
    spawn_consume, unique,
};

#[tokio::test]
async fn consumer_entrega_evento_tipado_ao_handler() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("consume");
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

    let before = std::time::SystemTime::now();

    bus.publish_event(AlunoMatriculado {
        aluno_id: "a-1".into(),
        curso: "matematica".into(),
    })
    .await
    .expect("publish confirmado");

    let received = next_with_timeout(&mut rx, Duration::from_secs(10)).await;
    assert_eq!(received.event.aluno_id, "a-1");
    assert_eq!(received.event.curso, "matematica");
    assert_eq!(received.metadata.event_type, "aluno.matriculado");
    let occurred = received.metadata.occurred_at.timestamp();
    let before_secs = before
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64;
    assert!(
        occurred >= before_secs && occurred <= before_secs + 60,
        "occurred_at preservado do produtor (occurred {occurred}, before {before_secs})"
    );

    handle.abort();
    connection.close().await.ok();
}

#[tokio::test]
async fn retry_republishes_com_atraso_e_sem_loop() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("retry");
    let config = common::config(&uri, &name);
    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");

    let registry = Arc::new(RabbitHandlerRegistry::new());
    let (tx, mut rx) = mpsc::unbounded_channel();
    let handler = RetryThenCollectHandler {
        failures: AtomicUsize::new(1),
        tx,
    };
    registry.register::<AlunoMatriculado, _>("aluno.matriculado", handler);

    let consumer_config = RabbitConsumerConfig::builder()
        .queue(format!("{name}.queue"))
        .retry(RetryPolicy::new(2, Duration::from_millis(150), 1.0))
        .build()
        .expect("config consumer");
    let consumer = Arc::new(
        RabbitMqConsumer::connect(&connection, consumer_config, registry)
            .await
            .expect("consumer"),
    );

    let bus = common::bus_for(&connection, &config).await;
    let handle = spawn_consume(consumer);

    bus.publish_event(AlunoMatriculado {
        aluno_id: "a-1".into(),
        curso: "matematica".into(),
    })
    .await
    .expect("publish");

    // primeira tentativa falha (retryable), segunda tem sucesso
    let received = next_with_timeout(&mut rx, Duration::from_secs(10)).await;
    assert_eq!(received.event.aluno_id, "a-1");

    // sem loop de redelivery: após sucesso, a fila sinaliza vazia
    let channel = connection.channel().await.expect("canal");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let passive = QueueDeclareOptions {
        passive: true,
        ..Default::default()
    };
    let queue_state = channel
        .queue_declare(
            format!("{name}.queue").as_str().into(),
            passive,
            FieldTable::default(),
        )
        .await
        .expect("declaração passiva");
    assert_eq!(
        queue_state.message_count(),
        0,
        "mensagem deve ser confirmada (ack)"
    );

    handle.abort();
    connection.close().await.ok();
}

#[tokio::test]
async fn falha_nao_retentavel_vai_para_dlq() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("dlq");
    let config = common::config(&uri, &name);
    let dlx = format!("{name}.dlx");
    let dlq = format!("{name}.dlq");
    let dead_letter = DeadLetterConfig::default().enabled(true).names(&dlx, &dlq);

    let registry = Arc::new(RabbitHandlerRegistry::new());
    registry.register::<AlunoMatriculado, _>("aluno.matriculado", AlwaysFailHandler);

    let consumer_config = RabbitConsumerConfig::builder()
        .queue(format!("{name}.queue"))
        .retry(RetryPolicy::disabled())
        .dead_letter(dead_letter)
        .build()
        .expect("config consumer");

    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");
    let consumer = Arc::new(
        RabbitMqConsumer::connect(&connection, consumer_config, registry)
            .await
            .expect("consumer"),
    );

    let bus = common::bus_for(&connection, &config).await;
    let handle = spawn_consume(consumer);

    bus.publish_event(AlunoMatriculado {
        aluno_id: "dlq-1".into(),
        curso: "x".into(),
    })
    .await
    .expect("publish");

    let channel = connection.channel().await.expect("canal");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut found = false;
    while tokio::time::Instant::now() < deadline {
        if let Some(delivery) = channel
            .basic_get(dlq.as_str().into(), BasicGetOptions::default())
            .await
            .expect("get dlq")
        {
            let decoded = JsonEnvelopeCodec
                .decode(&delivery.data)
                .expect("decodifica dlq");
            assert_eq!(decoded.payload["aluno_id"], serde_json::json!("dlq-1"));
            delivery
                .ack(BasicAckOptions::default())
                .await
                .expect("ack dlq");
            found = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(found, "mensagem rejeitada deveria ir para a DLQ");

    handle.abort();
    connection.close().await.ok();
}

#[tokio::test]
async fn evento_sem_handler_reject_vai_para_dlq() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("unknown-reject");
    let config = common::config(&uri, &name);
    let dlx = format!("{name}.dlx");
    let dlq = format!("{name}.dlq");
    let dead_letter = DeadLetterConfig::default().enabled(true).names(&dlx, &dlq);

    let registry = Arc::new(RabbitHandlerRegistry::new());

    let consumer_config = RabbitConsumerConfig::builder()
        .queue(format!("{name}.queue"))
        .bindings(["aluno.#"])
        .retry(RetryPolicy::disabled())
        .dead_letter(dead_letter)
        .on_unknown_event(UnknownEventPolicy::Reject)
        .build()
        .expect("config consumer");

    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");
    let consumer = Arc::new(
        RabbitMqConsumer::connect(&connection, consumer_config, registry)
            .await
            .expect("consumer"),
    );

    let bus = common::bus_for(&connection, &config).await;
    let handle = spawn_consume(consumer);

    // nenhum handler registrado para `aluno.desmatriculado`
    bus.publish_event(AlunoMatriculado {
        aluno_id: "unknown-1".into(),
        curso: "y".into(),
    })
    .await
    .expect("publish");

    let channel = connection.channel().await.expect("canal");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let mut found = false;
    while tokio::time::Instant::now() < deadline {
        if let Some(delivery) = channel
            .basic_get(dlq.as_str().into(), BasicGetOptions::default())
            .await
            .expect("get dlq")
        {
            assert_eq!(
                delivery.properties.kind().as_ref().map(ToString::to_string),
                Some("aluno.matriculado".into())
            );
            delivery
                .ack(BasicAckOptions::default())
                .await
                .expect("ack dlq");
            found = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        found,
        "evento sem handler com política Reject deveria ir à DLQ"
    );

    handle.abort();
    connection.close().await.ok();
}

#[tokio::test]
async fn evento_sem_handler_ack_descarta_sem_dlq() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };
    let name = unique("unknown-ack");
    let config = common::config(&uri, &name);
    let dlx = format!("{name}.dlx");
    let dlq = format!("{name}.dlq");
    let dead_letter = DeadLetterConfig::default().enabled(true).names(&dlx, &dlq);

    let registry = Arc::new(RabbitHandlerRegistry::new());

    let consumer_config = RabbitConsumerConfig::builder()
        .queue(format!("{name}.queue"))
        .bindings(["aluno.#"])
        .retry(RetryPolicy::disabled())
        .dead_letter(dead_letter)
        .on_unknown_event(UnknownEventPolicy::Ack)
        .build()
        .expect("config consumer");

    let connection = RabbitMqConnection::connect(config.clone())
        .await
        .expect("conexão");
    let consumer = Arc::new(
        RabbitMqConsumer::connect(&connection, consumer_config, registry)
            .await
            .expect("consumer"),
    );

    let bus = common::bus_for(&connection, &config).await;
    let handle = spawn_consume(consumer);

    bus.publish_event(AlunoMatriculado {
        aluno_id: "unknown-2".into(),
        curso: "z".into(),
    })
    .await
    .expect("publish");

    // com política Ack, nada deve chegar à DLQ
    tokio::time::sleep(Duration::from_millis(800)).await;
    let channel = connection.channel().await.expect("canal");
    let in_dlq = channel
        .basic_get(dlq.as_str().into(), BasicGetOptions::default())
        .await
        .expect("get dlq");
    assert!(in_dlq.is_none(), "evento desconhecido ackado não vai à DLQ");

    handle.abort();
    connection.close().await.ok();
}
