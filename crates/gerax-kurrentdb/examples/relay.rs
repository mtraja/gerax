//! Demonstra o fluxo completo `KurrentDB → Event Relay → NATS JetStream`.
//!
//! Exige `KURRENTDB_URL` e `NATS_URL` (com JetStream habilitado). O
//! exemplo processa um único evento e encerra.
//!
//! ```sh
//! KURRENTDB_URL='kurrentdb://admin:changeit@localhost:2113?tls=false' \
//! NATS_URL='nats://localhost:4222' \
//!     cargo run -p gerax-kurrentdb --example relay
//! ```

#[cfg(feature = "eda")]
use chrono::Utc;
#[cfg(feature = "eda")]
use gerax_eda::event_relay::{
    EventDispatcher, EventRelay, PublisherHandler, RelayOutcome, RelayRetryPolicy,
};
#[cfg(feature = "eda")]
use gerax_es::{EventId, EventMetadata, EventStore, StoredEvent, Version};
#[cfg(feature = "eda")]
use gerax_kurrentdb::{
    KurrentDbClient, KurrentDbEventSource, KurrentDbEventStore, KurrentDbSubscription,
    SubscriptionConfig, SubscriptionManager, SubscriptionStart,
};
#[cfg(feature = "eda")]
use gerax_nats::{
    JetStreamConfig, NatsClient, NatsConfig, NatsIntegrationPublisher, NatsJetStream, StreamStorage,
};

#[cfg(feature = "eda")]
const EVENT_TYPE: &str = "aluno.matriculado.demo";

#[cfg(feature = "eda")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let kurrentdb_url = match std::env::var("KURRENTDB_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        _ => {
            eprintln!("defina KURRENTDB_URL para executar este exemplo");
            return Ok(());
        }
    };
    let nats_url = match std::env::var("NATS_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        _ => {
            eprintln!("defina NATS_URL (com JetStream) para executar este exemplo");
            return Ok(());
        }
    };

    let client = KurrentDbClient::connect(&kurrentdb_url).await?;
    let store = KurrentDbEventStore::new(client.clone());
    let manager = SubscriptionManager::new(client.clone());

    let aggregate_id = format!("aluno-{}", uuid::Uuid::new_v4().simple());
    let stream = format!("Aluno-{aggregate_id}");
    let config =
        SubscriptionConfig::new(&stream, format!("demo-{}", uuid::Uuid::new_v4().simple()))
            .start_from(SubscriptionStart::Start)
            .max_retry_count(3);

    let evento = StoredEvent {
        event_id: EventId::new(),
        aggregate_id: aggregate_id.clone(),
        aggregate_type: "Aluno".into(),
        event_type: EVENT_TYPE.into(),
        version: Version::new(1),
        payload: serde_json::json!({ "aluno_id": aggregate_id }),
        metadata: EventMetadata::new(),
        occurred_at: Utc::now(),
    };

    store
        .append("Aluno", &aggregate_id, Version::initial(), vec![evento])
        .await?;
    manager.ensure(&config).await?;

    let prefix = format!("gt.demo-{}", uuid::Uuid::new_v4().simple());
    let nats_stream = format!("GERAX_DEMO_{}", uuid::Uuid::new_v4().simple());
    let nats_config = NatsConfig::builder()
        .url(&nats_url)
        .name("gerax-kurrentdb-relay-demo")
        .subject_prefix(&prefix)
        .jetstream(
            JetStreamConfig::new()
                .stream(&nats_stream)
                .storage(StreamStorage::Memory),
        )
        .build()?;
    let nats_client = NatsClient::connect(nats_config).await?;
    let jetstream = NatsJetStream::new(
        &nats_client,
        JetStreamConfig::new()
            .stream(&nats_stream)
            .storage(StreamStorage::Memory),
    );
    jetstream.ensure_stream().await?;

    let publisher = NatsIntegrationPublisher::from_client(&nats_client);
    let dispatcher = EventDispatcher::new().register(EVENT_TYPE, PublisherHandler::new(publisher));

    let source = KurrentDbEventSource::new(KurrentDbSubscription::new(client, config.clone())?);
    let mut relay = EventRelay::new(source, dispatcher).with_retry_policy(RelayRetryPolicy::new(3));

    match relay.step().await? {
        RelayOutcome::Processed(action) => {
            println!("evento processado; ação na origem: {action:?} (publicado antes do ACK)")
        }
        RelayOutcome::Closed => println!("origem encerrada"),
    }

    manager.delete(&config.stream, &config.group).await?;
    jetstream.delete_stream().await?;
    Ok(())
}

#[cfg(not(feature = "eda"))]
fn main() {
    eprintln!("este exemplo requer a feature `eda` do gerax-kurrentdb");
}
