# gerax-eda-rabbitmq

Adapter **RabbitMQ (AMQP 0-9-1)** para o [`gerax-eda`](../gerax-eda): publica e
consome `EventEnvelope`s através do client [`lapin`](https://crates.io/crates/lapin).

## Features

- Produção com **publisher confirm** (aguarda Ack do broker).
- Mapeamento `EventMetadata` → propriedades AMQP (`message_id`, `type`,
  `timestamp`, `correlation_id`, headers).
- Consumo com **at-least-once**: `ack` só após o handler retornar `Ok`;
  `Retryable` reenfileira (com limite via header `x-death`); falhas
  definitivas são descartadas.
- Registro de handlers por `event_type` (contrato externo do evento).
- Codec plugável ([`EventCodec`](src/codec.rs)); JSON por padrão.

## Exemplo

```sh
cargo run -p gerax-eda-rabbitmq --example aluno_matriculado
```

Requires a RabbitMQ at `amqp://guest:guest@localhost:5672`.

```rust
use gerax_eda_rabbitmq::{JsonEventCodec, RabbitMqConfig, RabbitMqEventBus};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let bus = RabbitMqEventBus::connect(RabbitMqConfig::default(), JsonEventCodec).await?;
// ... registre handlers no RabbitHandlerRegistry e crie o
// RabbitMqConsumer; veja examples/aluno_matriculado.rs
# Ok(())
# }
```

## Mapeamento

| AMQP property              | `EventMetadata`                 |
|----------------------------|---------------------------------|
| `message_id`               | `EventEnvelope.id` (UUID)       |
| `type`                     | `event_type`                    |
| `timestamp`                | `occurred_at` (UNIX)            |
| `correlation_id`           | `correlation_id`                |
| `headers["x-causation-id"]`| `causation_id`                  |
| `headers["x-<key>"]`       | `headers[<key>]`                |

`routing_key` = `event_type` (exchange tópico por padrão).

## Semântica

Semântica **at-least-once**: consumidores devem ser idempotentes por
`EventEnvelope.id`. `gerax_eda::EventBus` não é implementada porque exige
`E: Serialize`/`Deserialize`; o bus expõe `publish` inerente com o mesmo
contrato (veja a doc da crate).