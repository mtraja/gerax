//! Testes de integração: conexão (`§24`).
//!
//! Requer um RabbitMQ real. Skip quando `RABBITMQ_URI` não está setada.

mod common;

use common::{connection_if_available, gate_rabbitmq};
use gerax_rabbitmq::RabbitMqConnection;

#[tokio::test]
async fn conecta_cria_canal_e_fecha() {
    let Some(uri) = gate_rabbitmq() else {
        return;
    };

    let connection = RabbitMqConnection::connect(common::config(&uri, "gate-connect"))
        .await
        .expect("conexão com o broker de teste");

    let channel = connection.channel().await.expect("canal criado");
    assert!(channel.status().connected());
    drop(channel);

    connection
        .close()
        .await
        .expect("conexão fechada graciosamente");
}

#[tokio::test]
async fn nome_da_conexao_e_exposto_na_config() {
    let Some(connection) = connection_if_available().await else {
        return;
    };

    assert!(connection.config().connection_name.is_some());
    connection.close().await.ok();
}
