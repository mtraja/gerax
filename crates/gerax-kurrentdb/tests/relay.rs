#![cfg(feature = "eda")]
//! Testes de integração do fluxo completo:
//!
//! ```text
//! KurrentDB → Persistent Subscription → Event Relay → NATS JetStream
//! ```
//!
//! Exigem `KURRENTDB_URL` **e** `NATS_URL` (JetStream habilitado). Sem
//! as duas variáveis os testes são ignorados.

mod common;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use gerax_eda::event_relay::{
    DeliveryAction, EventDispatcher, EventRelay, IntegrationEvent, IntegrationPublisher,
    PublisherHandler, RelayOutcome, RelayRetryPolicy,
};
use gerax_es::EventStore;
use gerax_es::version::Version;
use gerax_kurrentdb::{
    KurrentDbClient, KurrentDbEventSource, KurrentDbEventStore, KurrentDbSubscription,
    SubscriptionAck, SubscriptionConfig, SubscriptionManager, SubscriptionStart,
};
use gerax_nats::{
    JetStreamConfig, NatsClient, NatsConfig, NatsConsumer, NatsConsumerConfig, NatsDelivered,
    NatsIntegrationPublisher, NatsJetStream, StreamStorage,
};
use tokio::sync::mpsc;

use common::{DELIVERY_TIMEOUT, gate_relay, stored_event, unique};

const EVENT_TYPE: &str = "aluno.matriculado.relay";

struct Fixture {
    client: KurrentDbClient,
    store: KurrentDbEventStore,
    manager: SubscriptionManager,
    config: SubscriptionConfig,
    aggregate_id: String,
    nats_client: NatsClient,
    nats_stream: String,
    subject_prefix: String,
    publisher: NatsIntegrationPublisher,
}

impl Fixture {
    async fn new(kurrentdb_url: &str, nats_url: &str) -> Self {
        let client = KurrentDbClient::connect(kurrentdb_url)
            .await
            .expect("conexão com KurrentDB");
        let store = KurrentDbEventStore::new(client.clone());
        let manager = SubscriptionManager::new(client.clone());

        let aggregate_id = unique("aluno");
        let stream = format!("GtAluno-{aggregate_id}");
        let config = SubscriptionConfig::new(stream, unique("gt-relay"))
            .buffer_size(16)
            .max_retry_count(2)
            .start_from(SubscriptionStart::Start)
            .message_timeout_secs(5)
            .reconnect_delay_ms(100);
        manager.ensure(&config).await.expect("grupo do relay");

        let subject_prefix = format!("gt.relay-{}", uuid::Uuid::new_v4().simple());
        let nats_stream = format!("GERAX_RELAY_{}", uuid::Uuid::new_v4().simple());
        let nats_config = NatsConfig::builder()
            .url(nats_url)
            .name(format!("gerax-relay-test-{subject_prefix}"))
            .subject_prefix(&subject_prefix)
            .jetstream(
                JetStreamConfig::new()
                    .stream(&nats_stream)
                    .storage(StreamStorage::Memory),
            )
            .build()
            .expect("config NATS de teste");
        let nats_client = NatsClient::connect(nats_config)
            .await
            .expect("conexão com NATS");
        let jetstream = NatsJetStream::new(
            &nats_client,
            JetStreamConfig::new()
                .stream(&nats_stream)
                .storage(StreamStorage::Memory),
        );
        jetstream.ensure_stream().await.expect("stream JetStream");

        let publisher = NatsIntegrationPublisher::from_client(&nats_client);

        Self {
            client,
            store,
            manager,
            config,
            aggregate_id,
            nats_client,
            nats_stream,
            subject_prefix,
            publisher,
        }
    }

    fn subscription(&self) -> KurrentDbSubscription {
        KurrentDbSubscription::new(self.client.clone(), self.config.clone()).expect("subscription")
    }

    async fn append(&self) -> gerax_es::EventId {
        let stored = stored_event(
            "GtAluno",
            &self.aggregate_id,
            1,
            EVENT_TYPE,
            serde_json::json!({ "aggregate_id": self.aggregate_id }),
        );
        let event_id = stored.event_id.clone();
        self.store
            .append(
                "GtAluno",
                &self.aggregate_id,
                Version::initial(),
                vec![stored],
            )
            .await
            .expect("append");
        event_id
    }

    fn subject(&self) -> String {
        format!("{}.{}", self.subject_prefix, EVENT_TYPE)
    }

    async fn cleanup(&self) {
        let _ = self
            .manager
            .delete(&self.config.stream, &self.config.group)
            .await;
        let jetstream = NatsJetStream::new(
            &self.nats_client,
            JetStreamConfig::new()
                .stream(&self.nats_stream)
                .storage(StreamStorage::Memory),
        );
        let _ = jetstream.delete_stream().await;
    }

    async fn wait_in_flight_zero(&self) {
        for _ in 0..100 {
            if let Ok(info) = self
                .manager
                .info(&self.config.stream, &self.config.group)
                .await
                && info.total_in_flight == 0
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("evento continuou em voo (ACK não confirmado)");
    }

    async fn stream_messages(&self) -> usize {
        let jetstream = NatsJetStream::new(
            &self.nats_client,
            JetStreamConfig::new()
                .stream(&self.nats_stream)
                .storage(StreamStorage::Memory),
        );
        let info = jetstream
            .context()
            .get_stream(&self.nats_stream)
            .await
            .expect("stream")
            .get_info()
            .await
            .expect("stream info");
        info.state.messages as usize
    }
}

async fn next_published(client: &NatsClient, stream: &str, subject: &str) -> NatsDelivered {
    let config = NatsConsumerConfig::builder()
        .stream(stream)
        .durable_name(unique("relay-consumer"))
        .filter_subjects([subject.to_string()])
        .ack_wait(Duration::from_secs(2))
        .max_deliver(1)
        .build()
        .expect("config do consumidor");
    let consumer = Arc::new(NatsConsumer::new(client, config).expect("consumer JetStream"));
    let (tx, mut rx) = mpsc::unbounded_channel();

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

    let delivered = tokio::time::timeout(DELIVERY_TIMEOUT, rx.recv())
        .await
        .expect("mensagem publicada dentro do timeout")
        .expect("canal aberto");
    handle.abort();
    delivered
}

#[tokio::test]
async fn relay_publica_antes_do_ack_e_preserva_identidade() {
    let Some((kurrentdb_url, nats_url)) = gate_relay() else {
        return;
    };
    let fixture = Fixture::new(&kurrentdb_url, &nats_url).await;
    let event_id = fixture.append().await;

    let dispatcher = EventDispatcher::new()
        .register(EVENT_TYPE, PublisherHandler::new(fixture.publisher.clone()));
    let source = KurrentDbEventSource::new(fixture.subscription());
    let mut relay = EventRelay::new(source, dispatcher).with_retry_policy(RelayRetryPolicy::new(2));

    let outcome = relay.step().await.expect("step do relay");
    assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Ack));
    assert_eq!(relay.stats().acked, 1);

    // A mensagem foi confirmada no JetStream antes do ACK.
    let delivered = next_published(
        &fixture.nats_client,
        &fixture.nats_stream,
        &fixture.subject(),
    )
    .await;
    assert_eq!(delivered.envelope.event_id(), event_id);
    assert_eq!(delivered.envelope.event_type, EVENT_TYPE);
    assert_eq!(
        delivered.envelope.aggregate_id.as_deref(),
        Some(fixture.aggregate_id.as_str())
    );
    assert_eq!(
        delivered.envelope.aggregate_type.as_deref(),
        Some("GtAluno")
    );
    assert_eq!(delivered.envelope.stream_revision, Some(0));
    assert_eq!(delivered.envelope.version, Some(1));

    fixture.wait_in_flight_zero().await;
    fixture.cleanup().await;
}

#[tokio::test]
async fn falha_no_nats_nao_confirma_e_permite_reentrega() {
    let Some((kurrentdb_url, nats_url)) = gate_relay() else {
        return;
    };
    let fixture = Fixture::new(&kurrentdb_url, &nats_url).await;
    let event_id = fixture.append().await;

    // Cliente JetStream sem stream criada: a publicação falha de forma
    // transitória e o relay não pode confirmar o evento na origem.
    let failing_prefix = format!("gt.relay-fail-{}", uuid::Uuid::new_v4().simple());
    let failing_config = NatsConfig::builder()
        .url(&nats_url)
        .name(format!("gerax-relay-fail-{failing_prefix}"))
        .subject_prefix(&failing_prefix)
        .jetstream(
            JetStreamConfig::new()
                .stream(format!("GERAX_MISSING_{}", uuid::Uuid::new_v4().simple()))
                .storage(StreamStorage::Memory),
        )
        .build()
        .expect("config NATS inválida");
    let failing_client = NatsClient::connect(failing_config)
        .await
        .expect("conexão com NATS");
    let failing_publisher = NatsIntegrationPublisher::from_client(&failing_client);

    let dispatcher =
        EventDispatcher::new().register(EVENT_TYPE, PublisherHandler::new(failing_publisher));
    let source = KurrentDbEventSource::new(fixture.subscription());
    let mut relay = EventRelay::new(source, dispatcher).with_retry_policy(RelayRetryPolicy::new(2));

    let outcome = relay.step().await.expect("step do relay");
    assert_eq!(outcome, RelayOutcome::Processed(DeliveryAction::Retry));
    assert_eq!(relay.stats().retried, 1);

    // O evento continua na origem e é reentregue a uma nova sessão.
    let subscription = relay.into_source().into_subscription();
    let mut session = subscription.subscribe().await.expect("nova sessão");
    let redelivered = tokio::time::timeout(DELIVERY_TIMEOUT, session.next_event())
        .await
        .expect("reentrega dentro do timeout")
        .expect("evento reentregue");
    assert_eq!(redelivered.id, event_id);
    assert!(redelivered.retry_count >= 1);

    session
        .ack(&redelivered, SubscriptionAck::Ack)
        .await
        .expect("ack após reentrega");

    fixture.cleanup().await;
}

#[tokio::test]
async fn publicacao_duplicada_e_deduplicada_pelo_msg_id() {
    let Some((kurrentdb_url, nats_url)) = gate_relay() else {
        return;
    };
    let fixture = Fixture::new(&kurrentdb_url, &nats_url).await;

    let event = IntegrationEvent::new(
        gerax_es::EventId::new(),
        EVENT_TYPE,
        &fixture.aggregate_id,
        "GtAluno",
        0,
        Utc::now(),
        serde_json::json!({ "aggregate_id": fixture.aggregate_id }),
    );

    fixture
        .publisher
        .publish(&event)
        .await
        .expect("primeira publicação");
    fixture
        .publisher
        .publish(&event)
        .await
        .expect("republicação aceita");

    assert_eq!(
        fixture.stream_messages().await,
        1,
        "Nats-Msg-Id deduplica a republicação dentro da janela"
    );

    let delivered = next_published(
        &fixture.nats_client,
        &fixture.nats_stream,
        &fixture.subject(),
    )
    .await;
    assert_eq!(delivered.envelope.event_id(), event.id);

    fixture.cleanup().await;
}
