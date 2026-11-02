#![cfg(feature = "kurrentdb")]
//! Testes de integração do adapter KurrentDB.
//!
//! Requer um KurrentDB real. Os testes são ignorados quando
//! `KURRENTDB_URL` não está definida.
//!
//! Exemplo:
//!
//! ```sh
//! KURRENTDB_URL='kurrentdb://admin:changeit@localhost:2113?tls=false' \
//!     cargo test -p gerax-es --features kurrentdb --test kurrentdb
//! ```

use chrono::Utc;
use gerax_es::adapters::kurrentdb::KurrentDbEventStore;
use gerax_es::{
    ConcurrencyError, EventId, EventMetadata, EventStore, EventStoreError, StoredEvent, Version,
};

fn connection_string() -> Option<String> {
    std::env::var("KURRENTDB_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn stream_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}

fn stored_event(aggregate_id: &str, version: u64, event_type: &str) -> StoredEvent {
    let mut metadata = EventMetadata::new();
    metadata.insert("correlation_id", "integration-test");

    StoredEvent {
        event_id: EventId::new(),
        aggregate_id: aggregate_id.to_string(),
        aggregate_type: "Student".to_string(),
        event_type: event_type.to_string(),
        version: Version::new(version),
        payload: serde_json::json!({ "name": "Ana" }),
        metadata,
        occurred_at: Utc::now(),
    }
}

#[tokio::test]
async fn load_de_stream_inexistente_retorna_vazio() {
    let Some(url) = connection_string() else {
        return;
    };

    let store = KurrentDbEventStore::connect(&url)
        .await
        .expect("conectar ao KurrentDB");
    let id = stream_id("missing");

    let events = store.load("Student", &id).await.expect("load");
    assert!(events.is_empty());
}

#[tokio::test]
async fn append_e_load_preservam_eventos_e_metadata() {
    let Some(url) = connection_string() else {
        return;
    };

    let store = KurrentDbEventStore::connect(&url)
        .await
        .expect("conectar ao KurrentDB");
    let id = stream_id("roundtrip");

    let events = vec![
        stored_event(&id, 1, "StudentCreated"),
        stored_event(&id, 2, "StudentRenamed"),
    ];

    store
        .append("Student", &id, Version::initial(), events.clone())
        .await
        .expect("append inicial");

    let loaded = store.load("Student", &id).await.expect("load");
    assert_eq!(loaded, events);
}

#[tokio::test]
async fn append_com_versao_esperada_incorreta_retorna_conflito() {
    let Some(url) = connection_string() else {
        return;
    };

    let store = KurrentDbEventStore::connect(&url)
        .await
        .expect("conectar ao KurrentDB");
    let id = stream_id("conflict");

    store
        .append(
            "Student",
            &id,
            Version::initial(),
            vec![stored_event(&id, 1, "StudentCreated")],
        )
        .await
        .expect("append inicial");

    let error = store
        .append(
            "Student",
            &id,
            Version::initial(),
            vec![stored_event(&id, 1, "StudentCreated")],
        )
        .await
        .expect_err("append concorrente deve falhar");

    match error {
        EventStoreError::Concurrency(ConcurrencyError { expected, actual }) => {
            assert_eq!(expected, Version::initial());
            assert_eq!(actual, Version::new(1));
        }
        other => panic!("esperado EventStoreError::Concurrency, obtido {other:?}"),
    }
}

#[tokio::test]
async fn append_subsequente_continua_a_stream() {
    let Some(url) = connection_string() else {
        return;
    };

    let store = KurrentDbEventStore::connect(&url)
        .await
        .expect("conectar ao KurrentDB");
    let id = stream_id("append");

    store
        .append(
            "Student",
            &id,
            Version::initial(),
            vec![stored_event(&id, 1, "StudentCreated")],
        )
        .await
        .expect("append inicial");

    store
        .append(
            "Student",
            &id,
            Version::new(1),
            vec![stored_event(&id, 2, "StudentRenamed")],
        )
        .await
        .expect("append subsequente");

    let loaded = store.load("Student", &id).await.expect("load");
    let event_types: Vec<_> = loaded
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    assert_eq!(event_types, vec!["StudentCreated", "StudentRenamed"]);
}
