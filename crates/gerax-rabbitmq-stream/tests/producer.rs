//! Integração com um RabbitMQ real: publicação nos três modos,
//! batches e ciclo de vida do producer (§33).

mod common;

use std::time::Duration;

use common::{TestBroker, envelope, matriculado};
use gerax_rabbitmq_stream::{
    JsonEventSerializer, PublishMode, RabbitMqStreamError, RabbitMqStreamProducer,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn modo_confirmado_espera_o_broker() {
    let Some(broker) = TestBroker::open("prod-confirmed").await else {
        return;
    };

    let producer = broker.producer().await;
    assert_eq!(producer.mode(), PublishMode::Confirmed);
    assert!(!producer.is_closed());

    producer
        .publish(&envelope(matriculado("a-1")))
        .await
        .expect("publicação confirmada");

    producer.close().await.expect("producer fechado");
    assert!(producer.is_closed());
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn modo_assincrono_enfileira_e_confirma() {
    let Some(broker) = TestBroker::open("prod-async").await else {
        return;
    };

    let producer = broker
        .connection
        .producer(PublishMode::Async, 10, JsonEventSerializer)
        .await
        .expect("producer assíncrono");

    for id in ["a-1", "a-2", "a-3"] {
        producer
            .publish(&envelope(matriculado(id)))
            .await
            .expect("publicação enfileirada");
    }

    // `close()` drena as confirmações pendentes antes de encerrar.
    producer.close().await.expect("producer fechado");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lote_publica_todos_os_eventos() {
    let Some(broker) = TestBroker::open("prod-batch").await else {
        return;
    };

    let producer = broker.producer().await;
    let batch: Vec<_> = (0..50)
        .map(|index| envelope(matriculado(&format!("a-{index}"))))
        .collect();

    producer
        .publish_batch(&batch)
        .await
        .expect("lote publicado");

    // Lote vazio é no-op (não erro, não I/O).
    producer.publish_batch(&[]).await.expect("lote vazio");

    producer.close().await.expect("producer fechado");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publicar_depois_do_close_falha() {
    let Some(broker) = TestBroker::open("prod-closed").await else {
        return;
    };

    let producer = broker.producer().await;
    producer.close().await.expect("producer fechado");

    let result = producer.publish(&envelope(matriculado("a-1"))).await;
    assert!(matches!(result, Err(RabbitMqStreamError::ProducerClosed)));

    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_size_zero_e_rejeitado_sem_tocar_o_broker() {
    let Some(broker) = TestBroker::open("prod-batch-zero").await else {
        return;
    };

    let result = RabbitMqStreamProducer::<JsonEventSerializer>::connect(
        broker.connection.environment(),
        &broker.stream,
        PublishMode::Confirmed,
        0,
        JsonEventSerializer,
    )
    .await;

    assert!(result.is_err(), "batch_size 0 deve ser rejeitado");
    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn producer_compartilhado_aceita_publicacoes_concorrentes() {
    let Some(broker) = TestBroker::open("prod-concurrent").await else {
        return;
    };

    let producer = std::sync::Arc::new(broker.producer().await);

    let mut tasks = Vec::new();
    for worker in 0..4 {
        let producer = std::sync::Arc::clone(&producer);
        tasks.push(tokio::spawn(async move {
            for index in 0..5 {
                producer
                    .publish(&envelope(matriculado(&format!("w{worker}-{index}"))))
                    .await
                    .expect("publicação concorrente");
            }
        }));
    }

    for task in tasks {
        task.await.expect("task de publicação");
    }

    producer.close().await.expect("producer fechado");
    tokio::time::sleep(Duration::from_millis(100)).await;
    broker.cleanup().await;
}
