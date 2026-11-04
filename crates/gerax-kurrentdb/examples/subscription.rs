//! Demonstra uma Persistent Subscription com ACK explícito.
//!
//! ```sh
//! KURRENTDB_URL='kurrentdb://admin:changeit@localhost:2113?tls=false' \
//!     cargo run -p gerax-kurrentdb --example subscription
//! ```

use std::time::Duration;

use chrono::Utc;
use gerax_es::{EventId, EventMetadata, EventStore, StoredEvent, Version};
use gerax_kurrentdb::{
    DEFAULT_CONNECTION_STRING, KurrentDbClient, KurrentDbEventStore, KurrentDbSubscription,
    SubscriptionAck, SubscriptionConfig, SubscriptionManager, SubscriptionStart,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url =
        std::env::var("KURRENTDB_URL").unwrap_or_else(|_| DEFAULT_CONNECTION_STRING.to_string());
    let client = KurrentDbClient::connect(&url).await?;
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = format!("aluno-{}", uuid::Uuid::new_v4().simple());
    let stream = format!("Aluno-{aggregate_id}");
    let config =
        SubscriptionConfig::new(&stream, format!("demo-{}", uuid::Uuid::new_v4().simple()))
            .start_from(SubscriptionStart::Start)
            .max_retry_count(3)
            .message_timeout_secs(30);

    let evento = StoredEvent {
        event_id: EventId::new(),
        aggregate_id: aggregate_id.clone(),
        aggregate_type: "Aluno".into(),
        event_type: "AlunoMatriculado".into(),
        version: Version::new(1),
        payload: serde_json::json!({ "aluno_id": aggregate_id }),
        metadata: EventMetadata::new(),
        occurred_at: Utc::now(),
    };

    store
        .append("Aluno", &aggregate_id, Version::initial(), vec![evento])
        .await?;
    manager.ensure(&config).await?;

    let subscription = KurrentDbSubscription::new(client, config.clone())?;
    let mut session = subscription.subscribe().await?;

    let evento = tokio::time::timeout(Duration::from_secs(15), session.next_event()).await??;

    println!(
        "recebido {} do Aggregate {} (revisão {}, retry {})",
        evento.event_type, evento.aggregate_id, evento.stream_revision, evento.retry_count
    );

    // Só confirme depois do processamento ter sucesso.
    session.ack(&evento, SubscriptionAck::Ack).await?;
    println!("ACK enviado; grupo {}", config.group);

    manager.delete(&config.stream, &config.group).await?;
    Ok(())
}
