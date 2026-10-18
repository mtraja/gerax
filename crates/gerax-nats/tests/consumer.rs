#![cfg(feature = "jetstream")]
//! Testes de integração: consumidor JetStream — ACK, redelivery e DLQ
//! (`§12`–`§15`).
//!
//! Requer um NATS real com JetStream. Skip quando `NATS_URL` não está
//! setada.

mod common;

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use bytes::Bytes;
use common::{DELIVERY_TIMEOUT, envelope_eda, gate_nats, namespace};
use gerax_nats::{NatsConsumer, NatsConsumerConfig, NatsDelivered, NatsEnvelope, NatsPublisher};
use tokio::sync::mpsc;

fn consumer_config(ns: &common::TestNamespace, test: &str) -> NatsConsumerConfig {
    NatsConsumerConfig::builder()
        .durable_name(common::durable(test))
        .stream(&ns.stream)
        .filter_subjects([format!("{}.aluno.matriculado", ns.prefix)])
        .ack_wait(Duration::from_millis(300))
        .max_deliver(3)
        .batch_size(8)
        .dead_letter("tratamento")
        .build()
        .expect("config de consumidor de teste")
}

#[tokio::test]
async fn consumidor_duravel_processa_e_confirma_sem_redelivery() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("js-consumer-ok");
    let client = common::connect(&url, &ns, true).await;
    common::ensure_stream(&client, &ns).await;

    let publisher = NatsPublisher::new(&client);
    for aluno in ["a-1", "a-2"] {
        publisher
            .publish_envelope(&NatsEnvelope::from_event(&envelope_eda(aluno)).unwrap())
            .await
            .expect("publicado");
    }

    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = Arc::new(
        NatsConsumer::new(&client, consumer_config(&ns, "collector")).expect("consumer jetstream"),
    );
    let handle = tokio::spawn({
        let consumer = Arc::clone(&consumer);
        async move {
            consumer
                .consume(move |delivered: NatsDelivered| {
                    let tx = tx.clone();
                    async move {
                        tx.send(delivered)
                            .map_err(|_| gerax_nats::NatsError::Handler("canal fechado".into()))?;
                        Ok(())
                    }
                })
                .await
        }
    });

    let first = common::next_delivery(&mut rx).await;
    let second = common::next_delivery(&mut rx).await;
    handle.abort();

    assert_eq!(first.delivered, 1, "sem redelivery após sucesso");
    assert_eq!(second.delivered, 1, "sem redelivery após sucesso");
    assert_eq!(first.envelope.event_type(), "aluno.matriculado");
    assert_eq!(second.envelope.event_type(), "aluno.matriculado");

    let mut alunos: Vec<String> = [first, second]
        .map(|delivered| {
            delivered
                .envelope
                .payload::<common::AlunoMatriculado>()
                .expect("payload")
                .aluno_id
        })
        .into();
    alunos.sort();
    assert_eq!(alunos, vec!["a-1".to_string(), "a-2".to_string()]);

    common::cleanup_stream(&client, &ns).await;
}

#[tokio::test]
async fn handler_falha_e_jetstream_reentrega_ate_sucesso() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("js-consumer-redelivery");
    let client = common::connect(&url, &ns, true).await;
    common::ensure_stream(&client, &ns).await;

    let publisher = NatsPublisher::new(&client);
    publisher
        .publish_envelope(&NatsEnvelope::from_event(&envelope_eda("a-1")).unwrap())
        .await
        .expect("publicado");

    let (tx, mut rx) = mpsc::unbounded_channel();
    let attempts = Arc::new(AtomicUsize::new(0));
    let consumer = Arc::new(
        NatsConsumer::new(&client, consumer_config(&ns, "retry")).expect("consumer jetstream"),
    );
    let handle = tokio::spawn({
        let consumer = Arc::clone(&consumer);
        let handler = common::FailOnceHandler {
            attempts: Arc::clone(&attempts),
            tx,
        };
        async move { consumer.consume_event(handler).await }
    });

    let envelope = common::next_envelope(&mut rx).await;
    handle.abort();

    assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(envelope.event.aluno_id, "a-1");
    assert_eq!(
        envelope
            .metadata
            .correlation_id
            .as_ref()
            .map(|c| c.as_str()),
        Some("fluxo-teste")
    );

    common::cleanup_stream(&client, &ns).await;
}

#[tokio::test]
async fn falha_apos_max_deliver_encaminha_para_dlq() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("js-consumer-dlq");
    let client = common::connect(&url, &ns, true).await;
    common::ensure_stream(&client, &ns).await;

    let dlq_subject = format!("{}.dlq.tratamento.aluno.matriculado", ns.prefix);
    let mut dlq_subscriber = common::next_subscription(&client, &dlq_subject).await;

    let publisher = NatsPublisher::new(&client);
    let envelope = envelope_eda("a-1");
    let expected_id = envelope.id.as_uuid().to_string();
    publisher
        .publish_envelope(&NatsEnvelope::from_event(&envelope).unwrap())
        .await
        .expect("publicado");

    let config = NatsConsumerConfig::builder()
        .durable_name(common::durable("dlq"))
        .stream(&ns.stream)
        .filter_subjects([format!("{}.aluno.matriculado", ns.prefix)])
        .ack_wait(Duration::from_millis(200))
        .max_deliver(1)
        .batch_size(4)
        .dead_letter("tratamento")
        .build()
        .expect("config dlq");
    let consumer = Arc::new(NatsConsumer::new(&client, config).expect("consumer"));
    let handle = tokio::spawn({
        let consumer = Arc::clone(&consumer);
        async move { consumer.consume_event(common::AlwaysFailHandler).await }
    });

    let message = common::next_message(&mut dlq_subscriber, DELIVERY_TIMEOUT).await;
    handle.abort();

    let dlq: gerax_nats::DlqMessage =
        serde_json::from_slice(&message.payload).expect("dlq message json");
    assert_eq!(dlq.event_type, "aluno.matriculado");
    assert_eq!(dlq.event_id, Some(expected_id));
    assert_eq!(dlq.delivered, 1);
    assert_eq!(dlq.payload["aluno_id"], "a-1");
    assert!(!dlq.error.is_empty());
    assert_eq!(dlq.correlation_id.as_deref(), Some("fluxo-teste"));

    common::cleanup_stream(&client, &ns).await;
}

#[tokio::test]
async fn payload_indecifrável_vai_para_dlq_sem_consumir_stream() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("js-consumer-poison");
    let client = common::connect(&url, &ns, true).await;
    common::ensure_stream(&client, &ns).await;

    let dlq_subject = format!("{}.dlq.tratamento.unknown", ns.prefix);
    let mut dlq_subscriber = common::next_subscription(&client, &dlq_subject).await;

    // Payload "cru" que não é um NatsEnvelope válido.
    client
        .publish(
            &format!("{}.aluno.matriculado", ns.prefix),
            Bytes::from_static(b"nao-e-envelope"),
            None,
        )
        .await
        .expect("publicado");

    let config = NatsConsumerConfig::builder()
        .durable_name(common::durable("poison"))
        .stream(&ns.stream)
        .filter_subjects([format!("{}.aluno.matriculado", ns.prefix)])
        .ack_wait(Duration::from_millis(200))
        .max_deliver(1)
        .batch_size(4)
        .dead_letter("tratamento")
        .build()
        .expect("config poison");
    let consumer = Arc::new(NatsConsumer::new(&client, config).expect("consumer"));
    let handle = tokio::spawn({
        let consumer = Arc::clone(&consumer);
        async move {
            consumer
                .consume(|_delivered: NatsDelivered| async { Ok(()) })
                .await
        }
    });

    let message = common::next_message(&mut dlq_subscriber, DELIVERY_TIMEOUT).await;
    handle.abort();

    let dlq: gerax_nats::DlqMessage =
        serde_json::from_slice(&message.payload).expect("dlq message json");
    assert_eq!(dlq.event_type, "unknown");
    assert!(dlq.event_id.is_none());
    // Payload não-JSON não é parseável: preservado como `null`.
    assert_eq!(dlq.payload, serde_json::Value::Null);

    common::cleanup_stream(&client, &ns).await;
}
