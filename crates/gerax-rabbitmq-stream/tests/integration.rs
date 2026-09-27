//! Cenário completo com RabbitMQ real: `RabbitMqStreamEventBus` +
//! subscriptions, shutdown ordenado e lifecycle das tasks (§33, §18).

mod common;

use std::time::Duration;

use common::{CollectHandler, TestBroker, gate_broker, next_with_timeout, unique};
use gerax_eda::{EventEnvelope, EventHandler, EventHandlerError};
use gerax_rabbitmq_stream::{
    ConsumerConfig, JsonEventSerializer, PublishMode, RabbitMqStreamConfig,
    RabbitMqStreamConnection, RabbitMqStreamEventBusBuilder, StartOffset, StreamConfig,
    StreamHandlerRegistry,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct AlunoCriado {
    aluno_id: String,
}

impl gerax_eda::Event for AlunoCriado {
    fn event_type(&self) -> &'static str {
        "aluno.criado"
    }
}

struct ProjectionHandler {
    tx: mpsc::UnboundedSender<EventEnvelope<AlunoCriado>>,
}

#[async_trait::async_trait]
impl EventHandler<AlunoCriado> for ProjectionHandler {
    async fn handle(&self, event: EventEnvelope<AlunoCriado>) -> Result<(), EventHandlerError> {
        self.tx
            .send(event)
            .map_err(|_| EventHandlerError::NonRetryable("canal fechado".into()))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fluxo_completo_publica_consome_e_faz_shutdown() {
    let Some(config) = gate_broker() else {
        return;
    };

    let stream = unique("e2e");
    let mut bus = RabbitMqStreamEventBusBuilder::new(config.clone(), StreamConfig::new(&stream))
        .publish_mode(PublishMode::Confirmed)
        .build()
        .await
        .expect("bus pronto (stream criado)");
    assert_eq!(bus.subscription_count(), 0);

    // Registro por `event_type` (string) — o mesmo contrato do
    // `gerax-rabbitmq`.
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (registry_tx, _registry_rx) = mpsc::unbounded_channel();
    let registry = StreamHandlerRegistry::new();
    registry.register::<AlunoCriado, _, _>(
        "aluno.criado",
        ProjectionHandler { tx: registry_tx },
        std::sync::Arc::new(JsonEventSerializer),
    );
    assert!(registry.contains("aluno.criado"));
    assert_eq!(registry.event_types(), vec!["aluno.criado".to_string()]);

    let consumer = bus
        .connection()
        .consumer(
            ConsumerConfig::builder()
                .stream(&stream)
                .consumer_name(unique("e2e-consumer"))
                .offset(StartOffset::First)
                .build()
                .expect("config de consumer"),
            JsonEventSerializer,
        )
        .await
        .expect("consumer");

    // O consumer entrega ao mesmo `event_type` registrado acima; o
    // handler usado aqui é o do teste (canal `rx`).
    consumer.register::<AlunoCriado, _>("aluno.criado", ProjectionHandler { tx });

    let subscription = consumer.subscribe().await.expect("subscription");
    assert!(subscription.is_running());
    assert_eq!(subscription.stream(), stream);
    bus.add_subscription(subscription);
    assert_eq!(bus.subscription_count(), 1);

    bus.publish(
        EventEnvelope::new(AlunoCriado {
            aluno_id: "a-42".into(),
        })
        .with_correlation_id("fluxo-e2e"),
    )
    .await
    .expect("publicação confirmada");

    let received = next_with_timeout(&mut rx, TIMEOUT).await;
    assert_eq!(received.event.aluno_id, "a-42");
    assert_eq!(
        received
            .metadata
            .correlation_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("fluxo-e2e")
    );

    // Shutdown: para a subscription e depois o producer.
    bus.shutdown().await.expect("shutdown do bus");

    RabbitMqStreamConnection::connect(config, StreamConfig::new(&stream))
        .await
        .expect("reconexão para cleanup")
        .delete_stream()
        .await
        .expect("stream de teste removido");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bus_sem_stream_criado_falha_no_producer() {
    let Some(config) = gate_broker() else {
        return;
    };

    let result = RabbitMqStreamEventBusBuilder::new(
        config.clone(),
        StreamConfig::new(unique("e2e-nostream")),
    )
    .create_stream(false)
    .build()
    .await;

    assert!(result.is_err(), "producer exige stream existente");

    let broker = TestBroker::connect(config, "e2e-nostream-check").await;
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_sem_subscription_e_idempotente() {
    let Some(config) = gate_broker() else {
        return;
    };

    let bus = RabbitMqStreamEventBusBuilder::new(config, StreamConfig::new(unique("e2e-shutdown")))
        .build()
        .await
        .expect("bus pronto");

    bus.shutdown().await.expect("shutdown sem subscriptions");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn subscription_parada_no_shutdown_do_bus() {
    let Some(config) = gate_broker() else {
        return;
    };

    let stream = unique("e2e-shutdown-sub");
    let mut bus = RabbitMqStreamEventBusBuilder::new(config.clone(), StreamConfig::new(&stream))
        .build()
        .await
        .expect("bus pronto");

    let (tx, _rx) = mpsc::unbounded_channel();
    let consumer = bus
        .connection()
        .consumer(
            ConsumerConfig::builder()
                .stream(&stream)
                .consumer_name(unique("e2e-shutdown-consumer"))
                .build()
                .expect("config"),
            JsonEventSerializer,
        )
        .await
        .expect("consumer");
    consumer.register::<common::AlunoMatriculado, _>("aluno.matriculado", CollectHandler::new(tx));

    let subscription = consumer.subscribe().await.expect("subscription");
    assert!(subscription.is_running());
    bus.add_subscription(subscription);

    bus.shutdown().await.expect("shutdown com subscription");

    RabbitMqStreamConnection::connect(config, StreamConfig::new(&stream))
        .await
        .expect("reconexão para cleanup")
        .delete_stream()
        .await
        .expect("stream removido");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_local_usa_a_porta_de_streams() {
    // Guarda contra o erro clássico de apontar para 5672 (AMQP).
    let config: RabbitMqStreamConfig = RabbitMqStreamConfig::local();
    assert_eq!(config.port, 5552);
    assert_eq!(config.host, "localhost");
}
