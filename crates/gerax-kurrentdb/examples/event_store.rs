//! Demonstra o Event Store do KurrentDB via porta `gerax_es::EventStore`.
//!
//! ```sh
//! KURRENTDB_URL='kurrentdb://admin:changeit@localhost:2113?tls=false' \
//!     cargo run -p gerax-kurrentdb --example event_store
//! ```

use chrono::Utc;
use gerax_es::{EventId, EventMetadata, EventStore, StoredEvent, Version};
use gerax_kurrentdb::{DEFAULT_CONNECTION_STRING, KurrentDbEventStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url =
        std::env::var("KURRENTDB_URL").unwrap_or_else(|_| DEFAULT_CONNECTION_STRING.to_string());
    let store = KurrentDbEventStore::connect(&url).await?;

    let aggregate_id = format!("aluno-{}", uuid::Uuid::new_v4().simple());
    let mut metadata = EventMetadata::new();
    metadata.insert("correlation_id", "exemplo-event-store");

    let evento = StoredEvent {
        event_id: EventId::new(),
        aggregate_id: aggregate_id.clone(),
        aggregate_type: "Aluno".into(),
        event_type: "AlunoMatriculado".into(),
        version: Version::new(1),
        payload: serde_json::json!({ "aluno_id": aggregate_id, "curso": "matematica" }),
        metadata,
        occurred_at: Utc::now(),
    };

    store
        .append("Aluno", &aggregate_id, Version::initial(), vec![evento])
        .await?;

    let historico = store.load("Aluno", &aggregate_id).await?;
    println!("stream Aluno-{aggregate_id}: {} evento(s)", historico.len());

    for evento in historico {
        println!(
            "  v{} {} {}",
            evento.version.value(),
            evento.event_type,
            evento.event_id.as_uuid()
        );
    }

    Ok(())
}
