//! Topologia RabbitMQ (`§5`, `§10`, `§19`).
//!
//! O `RabbitMqEventBus` **não assume que existe somente uma fila**: o
//! exchange é o mesmo para todos os produtores, e cada consumidor
//! declara e liga a sua própria fila às routing keys (`event_type`)
//! que lhe interessam.
//!
//! ```text
//!               gerax.events (topic)
//!                    │
//!        ┌───────────┼───────────┐
//!        │           │           │
//!        ▼           ▼           ▼
//!    queue A      queue B      queue C
//!    binding      binding      binding
//!    "aluno.*"    "aluno.*"    "matricula.*"
//! ```
//!
//! Quando a DLQ está habilitada (`§19`), a fila principal é declarada
//! com `x-dead-letter-exchange`/`x-dead-letter-routing-key` apontando
//! para o DLX, e a fila de dead-letter é criada e ligada.
//!
//! ```text
//! gerax.events ──► main.queue ──reject──► gerax.events.dlx ──► gerax.events.dlq
//! ```

use lapin::Channel;
use lapin::options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::{AMQPValue, FieldTable};

use crate::config::{DeadLetterConfig, RabbitConsumerConfig, RabbitMqConfig};

/// Argumento AMQP que roteia mensagens rejeitadas para o DLX.
const ARG_DEAD_LETTER_EXCHANGE: &str = "x-dead-letter-exchange";
/// Argumento AMQP que define a routing key usada na rejeição.
const ARG_DEAD_LETTER_ROUTING_KEY: &str = "x-dead-letter-routing-key";

/// Declara o exchange de eventos (idempotente).
///
/// Usado tanto pelo produtor quanto pelo consumidor com os mesmos
/// parâmetros, garantindo consistência da topologia.
pub(crate) async fn declare_exchange(
    channel: &Channel,
    config: &RabbitMqConfig,
) -> Result<(), lapin::Error> {
    channel
        .exchange_declare(
            config.exchange.as_str().into(),
            config.exchange_type.clone(),
            ExchangeDeclareOptions {
                durable: config.durable,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map(|_| ())
}

/// Declara o DLX (exchange de dead-letter) e a DLQ (fila), ligando a
/// fila ao exchange com a routing key configurada.
///
/// Retorna `Ok(())` quando `dl` não está habilitado (nada é criado).
pub(crate) async fn declare_dlx_and_dlq(
    channel: &Channel,
    dl: &DeadLetterConfig,
) -> Result<(), lapin::Error> {
    if !dl.enabled {
        return Ok(());
    }

    channel
        .exchange_declare(
            dl.exchange.as_str().into(),
            lapin::ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: dl.durable,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            dl.queue.as_str().into(),
            QueueDeclareOptions {
                durable: dl.durable,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            dl.queue.as_str().into(),
            dl.exchange.as_str().into(),
            dl.routing_key.as_str().into(),
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    Ok(())
}

/// Declara a fila de consumo, com os argumentos de dead-letter quando
/// habilitada (`§19`).
pub(crate) async fn declare_queue(
    channel: &Channel,
    config: &RabbitConsumerConfig,
) -> Result<String, lapin::Error> {
    let mut arguments = FieldTable::default();
    if config.dead_letter.enabled {
        arguments.insert(
            ARG_DEAD_LETTER_EXCHANGE.into(),
            AMQPValue::LongString(config.dead_letter.exchange.clone().into()),
        );
        arguments.insert(
            ARG_DEAD_LETTER_ROUTING_KEY.into(),
            AMQPValue::LongString(config.dead_letter.routing_key.clone().into()),
        );
    }

    let queue = channel
        .queue_declare(
            config.queue.as_str().into(),
            QueueDeclareOptions {
                durable: config.durable,
                exclusive: config.exclusive,
                auto_delete: config.auto_delete,
                ..Default::default()
            },
            arguments,
        )
        .await?;

    Ok(queue.name().to_string())
}

/// Liga a fila ao exchange para cada routing key (`§10`).
///
/// A fila de um consumidor não precisa receber **todos** os eventos do
/// exchange: ela é ligada apenas aos `event_type` registrados.
pub(crate) async fn bind_queue(
    channel: &Channel,
    queue_name: &str,
    exchange: &str,
    routing_keys: &[String],
) -> Result<(), lapin::Error> {
    for routing_key in routing_keys {
        channel
            .queue_bind(
                queue_name.into(),
                exchange.into(),
                routing_key.as_str().into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
    }
    Ok(())
}
