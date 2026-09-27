//! Integração com um RabbitMQ real: conexão, criação e ciclo de vida
//! do stream (§33).
//!
//! Gated por `RABBITMQ_STREAM_HOST` — veja `tests/common/mod.rs`.

mod common;

use common::TestBroker;
use gerax_rabbitmq_stream::{
    RabbitMqStreamConfig, RabbitMqStreamConnection, RabbitMqStreamError, StreamConfig,
    StreamOutcome,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn connect_cria_o_stream_e_reconhece_o_ja_existente() {
    let Some(broker) = TestBroker::open("conn-create").await else {
        return;
    };

    let outcome = broker
        .connection
        .ensure_stream()
        .await
        .expect("segundo ensure_stream");
    assert!(matches!(outcome, StreamOutcome::AlreadyExists));

    broker.cleanup().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn broker_fora_do_ar_vira_erro_de_conexao() {
    // Porta 1 recusada localmente: falha rápida, sem esperar timeout.
    let config = RabbitMqStreamConfig::builder()
        .host("127.0.0.1")
        .port(1)
        .build()
        .expect("config valida");

    let result = RabbitMqStreamConnection::connect(config, StreamConfig::new("sem-broker")).await;

    match result {
        Err(RabbitMqStreamError::Connection(_)) => {}
        Err(other) => panic!("categoria de erro inesperada: {other}"),
        Ok(_) => panic!("conexão com broker inexistente não deveria funcionar"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_stream_remove_o_stream_criado() {
    let Some(broker) = TestBroker::open("conn-delete").await else {
        return;
    };

    broker
        .connection
        .delete_stream()
        .await
        .expect("stream removido");

    // Recriar o mesmo nome funciona: o stream deixou de existir.
    let outcome = broker
        .connection
        .ensure_stream()
        .await
        .expect("stream recriado");
    assert!(matches!(outcome, StreamOutcome::Created));

    broker.cleanup().await;
}
