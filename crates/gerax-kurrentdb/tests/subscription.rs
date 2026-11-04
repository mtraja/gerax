//! Testes de integração das Persistent Subscriptions.
//!
//! `KURRENTDB_URL` é obrigatório; sem ele os testes são ignorados.
//! Cada teste usa stream, grupo e Aggregate únicos, portanto pode rodar
//! em paralelo contra o mesmo servidor.

mod common;

use std::time::Duration;

use common::{DELIVERY_TIMEOUT, gate_kurrentdb, stored_event, unique};
use gerax_es::EventStore;
use gerax_es::version::Version;
use gerax_kurrentdb::{
    GroupState, KurrentDbClient, KurrentDbEventStore, KurrentDbSubscription, SubscriptionAck,
    SubscriptionConfig, SubscriptionManager, SubscriptionSession, SubscriptionStart,
};

fn config(stream: &str, group: &str) -> SubscriptionConfig {
    SubscriptionConfig::new(stream, group)
        .buffer_size(16)
        .max_retry_count(3)
        .start_from(SubscriptionStart::Start)
        .message_timeout_secs(5)
        .reconnect_delay_ms(100)
}

async fn append(store: &KurrentDbEventStore, aggregate_id: &str, version: u64, event_type: &str) {
    let expected = Version::new(version.saturating_sub(1));
    store
        .append(
            "GtAluno",
            aggregate_id,
            expected,
            vec![stored_event(
                "GtAluno",
                aggregate_id,
                version,
                event_type,
                serde_json::json!({ "aggregate_id": aggregate_id }),
            )],
        )
        .await
        .expect("append de teste");
}

async fn receive(session: &mut SubscriptionSession) -> gerax_kurrentdb::SubscriptionEvent {
    tokio::time::timeout(DELIVERY_TIMEOUT, session.next_event())
        .await
        .expect("evento entregue dentro do timeout")
        .expect("evento recebido")
}

async fn wait_for(
    manager: &SubscriptionManager,
    config: &SubscriptionConfig,
    predicate: impl Fn(&gerax_kurrentdb::SubscriptionInfo) -> bool,
) {
    for _ in 0..100 {
        if let Ok(info) = manager.info(&config.stream, &config.group).await
            && predicate(&info)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("condição não atingida no tempo esperado");
}

async fn cleanup(manager: &SubscriptionManager, config: &SubscriptionConfig) {
    let _ = manager.delete(&config.stream, &config.group).await;
}

#[tokio::test]
async fn cria_grupo_conecta_e_confirma_evento_com_ack() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let client = KurrentDbClient::connect(&url).await.expect("conexão");
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = unique("aluno");
    let config = config(&format!("GtAluno-{aggregate_id}"), &unique("gt-ack"));

    assert_eq!(
        manager.ensure(&config).await.expect("ensure"),
        GroupState::Created
    );
    assert_eq!(
        manager.ensure(&config).await.expect("ensure idempotente"),
        GroupState::Existing
    );

    append(&store, &aggregate_id, 1, "AlunoMatriculado").await;

    let subscription = KurrentDbSubscription::new(client, config.clone()).expect("subscription");
    let mut session = subscription.subscribe().await.expect("subscribe");

    let event = receive(&mut session).await;
    assert_eq!(event.event_type, "AlunoMatriculado");
    assert_eq!(event.stream_revision, 0);
    assert_eq!(event.aggregate_type, "GtAluno");
    assert_eq!(event.aggregate_id, aggregate_id);

    session
        .ack(&event, SubscriptionAck::Ack)
        .await
        .expect("ack");

    wait_for(&manager, &config, |info| info.total_in_flight == 0).await;
    cleanup(&manager, &config).await;
}

#[tokio::test]
async fn retry_entrega_novamente_o_mesmo_evento() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let client = KurrentDbClient::connect(&url).await.expect("conexão");
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = unique("aluno");
    let config = config(&format!("GtAluno-{aggregate_id}"), &unique("gt-retry"));
    manager.ensure(&config).await.expect("ensure");
    append(&store, &aggregate_id, 1, "AlunoMatriculado").await;

    let subscription = KurrentDbSubscription::new(client, config.clone()).expect("subscription");
    let mut session = subscription.subscribe().await.expect("subscribe");

    let first = receive(&mut session).await;
    session
        .ack(&first, SubscriptionAck::Retry)
        .await
        .expect("retry");

    let redelivered = receive(&mut session).await;
    assert_eq!(redelivered.id, first.id);
    assert!(redelivered.retry_count >= 1);

    session
        .ack(&redelivered, SubscriptionAck::Ack)
        .await
        .expect("ack após retry");
    cleanup(&manager, &config).await;
}

#[tokio::test]
async fn park_retira_o_evento_do_fluxo() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let client = KurrentDbClient::connect(&url).await.expect("conexão");
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = unique("aluno");
    let config = config(&format!("GtAluno-{aggregate_id}"), &unique("gt-park"));
    manager.ensure(&config).await.expect("ensure");
    append(&store, &aggregate_id, 1, "AlunoInvalido").await;

    let subscription = KurrentDbSubscription::new(client, config.clone()).expect("subscription");
    let mut session = subscription.subscribe().await.expect("subscribe");

    let event = receive(&mut session).await;
    session
        .ack(&event, SubscriptionAck::Park)
        .await
        .expect("park");

    wait_for(&manager, &config, |info| info.parked_message_count >= 1).await;

    let next = tokio::time::timeout(Duration::from_secs(2), session.next_event()).await;
    assert!(
        next.is_err(),
        "evento parkeado não deve ser reentregue automaticamente"
    );

    cleanup(&manager, &config).await;
}

#[tokio::test]
async fn retoma_consumo_apos_desconexao() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let client = KurrentDbClient::connect(&url).await.expect("conexão");
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = unique("aluno");
    let config = config(&format!("GtAluno-{aggregate_id}"), &unique("gt-reconnect"));
    manager.ensure(&config).await.expect("ensure");
    append(&store, &aggregate_id, 1, "AlunoMatriculado").await;

    let subscription = KurrentDbSubscription::new(client, config.clone()).expect("subscription");
    let mut session = subscription.subscribe().await.expect("primeira sessão");
    let first = receive(&mut session).await;
    session
        .ack(&first, SubscriptionAck::Ack)
        .await
        .expect("ack");
    drop(session);

    append(&store, &aggregate_id, 2, "AlunoTransferido").await;

    let mut session = subscription.subscribe().await.expect("reconexão");
    let second = receive(&mut session).await;
    assert_eq!(second.id, first.id);
    assert_eq!(second.stream_revision, 1);
    assert_eq!(second.event_type, "AlunoTransferido");
    session
        .ack(&second, SubscriptionAck::Ack)
        .await
        .expect("ack");

    cleanup(&manager, &config).await;
}

#[tokio::test]
async fn multiplas_instancias_compartilham_o_mesmo_grupo() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let client = KurrentDbClient::connect(&url).await.expect("conexão");
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = unique("aluno");
    let config = config(&format!("GtAluno-{aggregate_id}"), &unique("gt-shared"));
    manager.ensure(&config).await.expect("ensure");

    let subscription = KurrentDbSubscription::new(client, config.clone()).expect("subscription");
    let mut first = subscription.subscribe().await.expect("instância 1");
    let mut second = subscription.subscribe().await.expect("instância 2");

    wait_for(&manager, &config, |info| info.connections >= 2).await;
    append(&store, &aggregate_id, 1, "AlunoMatriculado").await;

    let received = tokio::time::timeout(DELIVERY_TIMEOUT, async {
        tokio::select! {
            event = first.next_event() => Some(("primeira", event)),
            event = second.next_event() => Some(("segunda", event)),
        }
    })
    .await
    .expect("uma instância recebe")
    .expect("sem erro");

    let (_, event) = received;
    let event = event.expect("evento válido");
    assert_eq!(event.aggregate_id, aggregate_id);

    cleanup(&manager, &config).await;
}

#[tokio::test]
async fn grupos_independentes_recebem_o_mesmo_evento() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let client = KurrentDbClient::connect(&url).await.expect("conexão");
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = unique("aluno");
    let stream = format!("GtAluno-{aggregate_id}");
    let config_a = config(&stream, &unique("gt-group-a"));
    let config_b = config(&stream, &unique("gt-group-b"));

    manager.ensure(&config_a).await.expect("grupo A");
    manager.ensure(&config_b).await.expect("grupo B");
    append(&store, &aggregate_id, 1, "AlunoMatriculado").await;

    let sub_a = KurrentDbSubscription::new(client.clone(), config_a.clone()).expect("sub A");
    let sub_b = KurrentDbSubscription::new(client, config_b.clone()).expect("sub B");
    let mut session_a = sub_a.subscribe().await.expect("sessão A");
    let mut session_b = sub_b.subscribe().await.expect("sessão B");

    let event_a = receive(&mut session_a).await;
    let event_b = receive(&mut session_b).await;

    assert_eq!(event_a.id, event_b.id);
    assert_eq!(event_a.stream_revision, event_b.stream_revision);
    assert_eq!(event_a.aggregate_id, event_b.aggregate_id);

    cleanup(&manager, &config_a).await;
    cleanup(&manager, &config_b).await;
}
