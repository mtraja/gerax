#![cfg(feature = "jetstream")]
//! Testes de integração: porta [`EventBus`](gerax_eda::EventBus)
//! implementada com NATS (`§16`, `§28`).
//!
//! Requer um NATS real com JetStream. Skip quando `NATS_URL` não está
//! setada.

mod common;

use std::time::Duration;

use common::{gate_nats, namespace};
use gerax_eda::{EventBus, EventBusError};
use gerax_nats::{NatsClient, NatsEnvelope, NatsEventBus};

#[tokio::test]
async fn porta_publica_evento_no_subject_com_rastreabilidade() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("bus-port");
    let client = common::connect(&url, &ns, true).await;
    common::ensure_stream(&client, &ns).await;

    let subject = format!("{}.aluno.matriculado", ns.prefix);
    let mut subscriber = common::next_subscription(&client, &subject).await;

    let bus = NatsEventBus::from_client(&client);
    bus.publish(common::envelope_eda("a-1"))
        .await
        .expect("publish via porta");

    let message = common::next_message(&mut subscriber, Duration::from_secs(5)).await;
    let envelope: NatsEnvelope =
        serde_json::from_slice(&message.payload).expect("envelope de transporte");

    assert_eq!(envelope.event_type(), "aluno.matriculado");
    assert_eq!(envelope.payload["aluno_id"], "a-1");
    assert_eq!(envelope.correlation_id.as_deref(), Some("fluxo-teste"));
    assert_eq!(envelope.causation_id.as_deref(), Some("evento-anterior"));
    assert!(envelope.event_id.is_some());

    let headers = message.headers.expect("headers presentes");
    assert_eq!(
        headers.get("Nats-Msg-Id").expect("msg id").as_str(),
        envelope.event_id.as_deref().expect("event_id")
    );

    common::cleanup_stream(&client, &ns).await;
}

#[tokio::test]
async fn porta_rejeita_event_type_divergente_sem_tocar_no_broker() {
    let Some(url) = gate_nats() else {
        return;
    };
    // A validação da porta (`InvalidEvent`) acontece antes do
    // transporte — a stream sequer é criada neste teste.
    let ns = namespace("bus-port-invalid");
    let client = common::connect(&url, &ns, true).await;

    let bus = NatsEventBus::from_client(&client);
    let mut envelope = common::envelope_eda("a-1");
    envelope.metadata.event_type = "outro.evento".into();

    let err = bus.publish(envelope).await.unwrap_err();
    assert!(matches!(err, EventBusError::InvalidEvent(_)));
    assert!(err.to_string().contains("outro.evento"));
}

#[tokio::test]
async fn porta_com_jetstream_sem_stream_falha_visivel() {
    let Some(url) = gate_nats() else {
        return;
    };
    // Sem `ensure_stream`: o PubAck não existe e o erro de transporte
    // vira `PublishFailed` — nunca silêncio (`§28`).
    let ns = namespace("bus-port-sem-stream");
    let client = NatsClient::connect(common::js_config(&url, &ns))
        .await
        .expect("conexao");

    let bus = NatsEventBus::from_client(&client);
    let err = bus.publish(common::envelope_eda("a-1")).await.unwrap_err();
    assert!(matches!(err, EventBusError::PublishFailed(_)));
}
