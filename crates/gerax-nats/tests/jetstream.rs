#![cfg(feature = "jetstream")]
//! Testes de integração: streams e publicação JetStream (`§10`–`§11`).
//!
//! Requer um NATS real com JetStream (`nats-server -js`). Skip quando
//! `NATS_URL` não está setada.

mod common;

use bytes::Bytes;
use common::{envelope_eda, gate_nats, namespace};
use gerax_nats::{
    JetStreamConfig, NatsEnvelope, NatsJetStream, NatsPublisher, PublishMode, StreamStorage,
};

#[tokio::test]
async fn ensure_stream_e_idempotente_e_delete_e_explicito() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("js-stream");
    let client = common::connect(&url, &ns, true).await;

    let jetstream = common::ensure_stream(&client, &ns).await;
    // Segunda chamada não reescreve nem falha (idempotente).
    jetstream.ensure_stream().await.expect("idempotente");

    jetstream.delete_stream().await.expect("delete explicito");
    // Após deletar, o get_stream falha — nunca recriado silenciosamente.
    let context = NatsJetStream::new(
        &client,
        JetStreamConfig::new()
            .stream(&ns.stream)
            .storage(StreamStorage::Memory),
    );
    let result = context.context().get_stream(ns.stream.clone()).await;
    assert!(result.is_err(), "stream removida pelo delete explicito");
}

#[tokio::test]
async fn publisher_jetstream_publica_com_puback_na_stream() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("js-publish");
    let client = common::connect(&url, &ns, true).await;
    let jetstream = common::ensure_stream(&client, &ns).await;

    let publisher = NatsPublisher::new(&client);
    assert_eq!(publisher.mode(), PublishMode::JetStream);

    let envelope = NatsEnvelope::from_event(&envelope_eda("a-1")).unwrap();
    publisher.publish_envelope(&envelope).await.expect("puback");

    let info = jetstream
        .context()
        .get_stream(&ns.stream)
        .await
        .expect("stream")
        .get_info()
        .await
        .expect("stream info");
    assert!(info.state.messages >= 1, "mensagem persistida na stream");

    common::cleanup_stream(&client, &ns).await;
}

#[tokio::test]
async fn publisher_jetstream_sem_stream_falha_alto_e_visivel() {
    let Some(url) = gate_nats() else {
        return;
    };
    // Namespace sem `ensure_stream`: a publicação JetStream não tem
    // onde ser persistida e o erro é explícito — nunca silencioso.
    let ns = namespace("js-publish-sem-stream");
    let client = common::connect(&url, &ns, true).await;

    let publisher = NatsPublisher::new(&client);
    let envelope = NatsEnvelope::from_event(&envelope_eda("a-1")).unwrap();
    let err = publisher.publish_envelope(&envelope).await.unwrap_err();
    assert!(matches!(err, gerax_nats::NatsError::Publish(_)));
}

#[tokio::test]
async fn nats_msg_id_deduplica_republicacao_na_stream() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("js-dedup");
    let client = common::connect(&url, &ns, true).await;
    let jetstream = common::ensure_stream(&client, &ns).await;

    let envelope = NatsEnvelope::from_event(&envelope_eda("a-1")).unwrap();
    let payload = Bytes::from(serde_json::to_vec(&envelope).expect("envelope json"));

    let mut headers = async_nats::HeaderMap::new();
    headers.insert(
        "Nats-Msg-Id",
        envelope.event_id.clone().expect("event_id presente"),
    );

    let context = jetstream.context();
    let first = context
        .publish_with_headers(
            format!("{}.aluno.matriculado", ns.prefix),
            headers.clone(),
            payload.clone(),
        )
        .await
        .expect("primeiro publish aceito")
        .await
        .expect("primeiro puback");
    assert!(!first.duplicate);

    let second = context
        .publish_with_headers(format!("{}.aluno.matriculado", ns.prefix), headers, payload)
        .await
        .expect("segundo publish aceito")
        .await
        .expect("segundo puback");
    assert!(
        second.duplicate,
        "republicacao do mesmo event_id dentro da janela e deduplicada pelo broker"
    );

    common::cleanup_stream(&client, &ns).await;
}
