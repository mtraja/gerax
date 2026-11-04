//! Testes de integração do Event Store contra um KurrentDB real.
//!
//! `KURRENTDB_URL` é obrigatório; sem ele os testes são ignorados.

mod common;

use common::{gate_kurrentdb, stored_event, unique};
use gerax_es::EventStore;
use gerax_es::error::EventStoreError;
use gerax_es::event::StoredEvent;
use gerax_es::version::Version;
use gerax_kurrentdb::KurrentDbEventStore;

async fn store(url: &str) -> KurrentDbEventStore {
    KurrentDbEventStore::connect(url)
        .await
        .expect("conexão com KurrentDB")
}

#[tokio::test]
async fn persiste_e_recupera_eventos_em_ordem() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let store = store(&url).await;
    let aggregate_id = unique("aluno");
    let events = vec![
        stored_event(
            "Aluno",
            &aggregate_id,
            1,
            "AlunoMatriculado",
            serde_json::json!({ "aluno_id": aggregate_id }),
        ),
        stored_event(
            "Aluno",
            &aggregate_id,
            2,
            "AlunoTransferido",
            serde_json::json!({ "turma": "B" }),
        ),
    ];

    store
        .append("Aluno", &aggregate_id, Version::initial(), events.clone())
        .await
        .expect("append");

    let loaded = store.load("Aluno", &aggregate_id).await.expect("load");

    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].event_id, events[0].event_id);
    assert_eq!(loaded[0].version, Version::new(1));
    assert_eq!(loaded[1].event_id, events[1].event_id);
    assert_eq!(loaded[1].version, Version::new(2));
    assert_eq!(loaded[1].event_type, "AlunoTransferido");
    assert_eq!(loaded[1].payload, serde_json::json!({ "turma": "B" }));
    assert_eq!(
        loaded[0].metadata.get("correlation_id"),
        Some(&serde_json::Value::from("fluxo-teste"))
    );
}

#[tokio::test]
async fn append_em_duas_etapas_preserva_versao_esperada() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let store = store(&url).await;
    let aggregate_id = unique("aluno");
    let first = stored_event(
        "Aluno",
        &aggregate_id,
        1,
        "AlunoMatriculado",
        serde_json::json!({ "ordem": 1 }),
    );
    let second = stored_event(
        "Aluno",
        &aggregate_id,
        2,
        "AlunoTransferido",
        serde_json::json!({ "ordem": 2 }),
    );

    store
        .append("Aluno", &aggregate_id, Version::initial(), vec![first])
        .await
        .expect("primeiro append");
    store
        .append("Aluno", &aggregate_id, Version::new(1), vec![second])
        .await
        .expect("segundo append");

    let loaded = store.load("Aluno", &aggregate_id).await.expect("load");
    assert_eq!(
        loaded.iter().map(|e| e.version.value()).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[tokio::test]
async fn concorrencia_otimista_rejeita_versao_desatualizada() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let store = store(&url).await;
    let aggregate_id = unique("aluno");
    let first = stored_event(
        "Aluno",
        &aggregate_id,
        1,
        "AlunoMatriculado",
        serde_json::json!({}),
    );
    let stale = stored_event(
        "Aluno",
        &aggregate_id,
        1,
        "AlunoMatriculado",
        serde_json::json!({ "stale": true }),
    );

    store
        .append("Aluno", &aggregate_id, Version::initial(), vec![first])
        .await
        .expect("primeiro append");

    let error = store
        .append("Aluno", &aggregate_id, Version::initial(), vec![stale])
        .await
        .expect_err("a segunda gravação deve conflitar");

    assert!(matches!(error, EventStoreError::Concurrency(_)));
}

#[tokio::test]
async fn stream_inexistente_retorna_vazio() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let store = store(&url).await;

    let events = store
        .load("Aluno", &unique("inexistente"))
        .await
        .expect("load de stream inexistente");

    assert!(events.is_empty());
}

#[tokio::test]
async fn append_vazio_e_idempotente() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let store = store(&url).await;
    let aggregate_id = unique("aluno");

    store
        .append(
            "Aluno",
            &aggregate_id,
            Version::initial(),
            Vec::<StoredEvent>::new(),
        )
        .await
        .expect("append vazio");

    assert!(store.load("Aluno", &aggregate_id).await.unwrap().is_empty());
}

#[tokio::test]
async fn sequencia_invalida_falha_antes_do_servidor() {
    let Some(url) = gate_kurrentdb() else {
        return;
    };
    let store = store(&url).await;
    let aggregate_id = unique("aluno");
    let event = stored_event(
        "Aluno",
        &aggregate_id,
        2,
        "AlunoMatriculado",
        serde_json::json!({}),
    );

    let error = store
        .append("Aluno", &aggregate_id, Version::initial(), vec![event])
        .await
        .expect_err("sequência começa em 2, esperado 1");

    assert!(matches!(error, EventStoreError::InvalidSequence(_)));
}
