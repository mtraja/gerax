#![cfg(feature = "jetstream")]
//! Testes de integração: Core NATS (`§7`, `§10`, `§17`).
//!
//! Requer um NATS real. Skip quando `NATS_URL` não está setada.

mod common;

use std::collections::HashSet;
use std::time::Duration;

use bytes::Bytes;
use common::{gate_nats, namespace};
use futures::StreamExt;

#[tokio::test]
async fn core_publicacao_entrega_payload_e_headers() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("core-pubsub");
    let client = common::connect(&url, &ns, false).await;

    let subject = format!("{}.aluno.matriculado", ns.prefix);
    let mut subscriber = common::next_subscription(&client, &subject).await;

    let payload = Bytes::from_static(br#"{"aluno_id":"a-1"}"#);
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("Nats-Msg-Id", "msg-core-1");
    headers.insert("x-correlation-id", "fluxo-1");
    client
        .publish(&subject, payload.clone(), Some(headers))
        .await
        .expect("publish core");

    let message = common::next_message(&mut subscriber, Duration::from_secs(5)).await;
    assert_eq!(message.subject.as_str(), subject);
    assert_eq!(message.payload, payload);

    let headers = message.headers.expect("headers presentes");
    assert_eq!(
        headers.get("Nats-Msg-Id").expect("msg id").as_str(),
        "msg-core-1"
    );
    assert_eq!(
        headers
            .get("x-correlation-id")
            .expect("correlacao")
            .as_str(),
        "fluxo-1"
    );
}

#[tokio::test]
async fn queue_group_distribui_entregas_sem_duplicar() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("core-queue");
    let client = common::connect(&url, &ns, false).await;

    let subject = format!("{}.aluno.matriculado", ns.prefix);
    let group = "tratamento";
    let mut member_a = client
        .queue_subscribe(&subject, group)
        .await
        .expect("queue member a");
    let mut member_b = client
        .queue_subscribe(&subject, group)
        .await
        .expect("queue member b");

    const TOTAL: usize = 6;
    for index in 0..TOTAL {
        let payload = Bytes::from(format!("msg-{index}"));
        client
            .publish(&subject, payload, None)
            .await
            .expect("publish");
    }

    let mut received: Vec<String> = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while received.len() < TOTAL {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        let message = tokio::select! {
            message = member_a.next() => message,
            message = member_b.next() => message,
            _ = tokio::time::sleep(left) => panic!("timeout aguardando entregas do queue group"),
        };
        let message = message.expect("subscription aberta");
        received.push(String::from_utf8_lossy(&message.payload).into_owned());
    }

    // Cada mensagem chega exatamente uma vez ao grupo (sem duplicar).
    let unique: HashSet<&str> = received.iter().map(String::as_str).collect();
    assert_eq!(received.len(), TOTAL);
    assert_eq!(unique.len(), TOTAL);
}

#[tokio::test]
async fn request_reply_faz_roundtrip() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("core-request");
    let client = common::connect(&url, &ns, false).await;

    let subject = format!("{}.aluno.existe", ns.prefix);
    let mut responder = common::next_subscription(&client, &subject).await;
    let connection = client.connection().clone();

    let echo = tokio::spawn(async move {
        let request = common::next_message(&mut responder, Duration::from_secs(5)).await;
        let reply = request.reply.expect("request tem reply subject");
        let body = format!("pong:{}", String::from_utf8_lossy(&request.payload));
        connection
            .publish(reply, Bytes::from(body))
            .await
            .expect("reply publicado");
    });

    let response = client
        .request(&subject, Bytes::from_static(b"ping"))
        .await
        .expect("request/reply");
    assert_eq!(response, Bytes::from_static(b"pong:ping"));
    echo.await.expect("responder concluido");
}

#[tokio::test]
async fn request_sem_responder_retorna_erro_tipado() {
    let Some(url) = gate_nats() else {
        return;
    };
    let ns = namespace("core-request-sem-responder");
    let client = common::connect(&url, &ns, false).await;

    let subject = format!("{}.aluno.nao-existe", ns.prefix);
    let err = client
        .request(&subject, Bytes::from_static(b"ping"))
        .await
        .unwrap_err();
    assert!(matches!(err, gerax_nats::NatsError::Request(_)));
}
