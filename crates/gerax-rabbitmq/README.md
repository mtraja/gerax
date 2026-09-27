# gerax-rabbitmq

Adapter de infraestrutura que transporta eventos do ecossistema **Gerax**
([`gerax-eda`](../gerax-eda), [`gerax-es`](../gerax-es)) via **RabbitMQ**
(protocolo AMQP 0-9-1).

```text
Aplicação ──► gerax-eda (EventBus, EventHandler)
                        │
                        ▼
              gerax-rabbitmq          RabbitMQ
              ┌──────────────┐        ┌──────────────────┐
              │ publish     │───────►│ gerax.events      │─ topic exchange
              │ consume     │◄───────│  school.alunos    │─ main queue
              │ retry/DLQ   │        │  gerax.events.dlq │─ dead-letter
              └──────────────┘        └──────────────────┘
```

O domínio **não** conhece RabbitMQ: `gerax-eda` define eventos, envelopes e
handlers; esta crate entrega e recebe essas estruturas no broker.

## Requisitos

- Rust (edition conforme o workspace);
- RabbitMQ 3.x (a topologia de desenvolvimento usa `rabbitmq:management`);
- dependências: `gerax-eda`, `gerax-es`, `lapin`, `tokio`, `serde`,
  `serde_json`, `chrono`, `uuid`, `async-trait`, `thiserror`, `tracing`.

## Broker local

O `compose.yaml` na raiz do workspace sobe um RabbitMQ de desenvolvimento:

```console
docker compose up -d       # ou: podman-compose up -d
```

- AMQP: `amqp://gerax:gerax@localhost:5672/%2f`
- Management UI: http://localhost:15672 (`gerax`/`gerax`)

> `guest:guest` e credenciais fixas são **apenas** para desenvolvimento
> (`§39`). Em produção a URI deve vir de configuração segura.

## Bootstrap

O adapter funciona com uma única [`RabbitMqConnection`] reutilizada por
produtores e consumidores ([`RabbitMqEventBus`], [`RabbitMqConsumer`]):

```rust,no_run
use gerax_rabbitmq::{RabbitMqConfig, RabbitMqConnection};

let config = RabbitMqConfig::new("amqp://gerax:gerax@localhost:5672/%2f", "gerax.events");
let connection = RabbitMqConnection::connect(config.clone()).await?;
```

A conexão declara **um único exchange** `topic` (durável) com o mesmo nome e
parâmetros em qualquer lado que a use — topologia consistente por
configuração.

## Publish-Subscribe

```rust,no_run
use gerax_rabbitmq::{RabbitMqEventBus, RabbitMqConnection, RabbitMqConfig};

let config = RabbitMqConfig::new(uri, "gerax.events");
let connection = RabbitMqConnection::connect(config.clone()).await?;
let bus = RabbitMqEventBus::from_connection(&connection, config).await?;

bus.publish(AlunoMatriculado { aluno_id: "a-1".into() }).await?;
```

Cada evento vira um [`EventEnvelope`] novo e é roteado pelo
`event_type` (routing key). `publish_envelope` permite preservar
`correlation_id`/`causation_id`/headers do fluxo; `publish_message`
consome envelopes de transporte já existentes (integrações de Outbox).

## Point-to-Point

Cada `RabbitConsumerConfig` aponta para **uma fila**. Quando `bindings`
está vazia, a fila é ligada aos `event_type` registrados no
[`RabbitHandlerRegistry`]; um consumidor por unidade lógica (worker)
dá o comportamento de competição:

```rust,no_run
let config = RabbitConsumerConfig::builder()
    .queue("school.alunos")
    .prefetch_count(50)
    .build()?;

let consumer = RabbitMqConsumer::connect(&connection, config, registry).await?;
consumer.consume().await?; // loop: entrega → handler → ack
```

## At-Least-Once e ACK

O ACK ocorre **somente após** `handle()` retornar `Ok` (nunca antes do
processamento). Falha retryável acima do limite e falha não-retryável
**rejeitam** a mensagem (`requeue = false`) → DLQ quando configurada.
Payload malformado também é rejeitado.

O [`MessageEnvelope`] carrega `event_id` para **idempotência** no
handler; `correlation_id`/`causation_id` viajam preservados entre
produtor e consumidor.

## Retry configurável

[`RetryPolicy`] é exponencial e determinística, sem
`requeue = true` infinito: o consumidor **re-publica após o atraso e só
então reconhece** a original:

```rust,no_run
use gerax_rabbitmq::RetryPolicy;

// tentativa 1 falha → +1s → tentativa 2 → falha → +2s → tentativa 3 → DLQ
let retry = RetryPolicy::new(3, std::time::Duration::from_secs(1), 2.0);
```

Padrão: uma única tentativa (sem retry).

## DLQ

```rust,no_run
use gerax_rabbitmq::{DeadLetterConfig, RabbitConsumerConfig};

let config = RabbitConsumerConfig::builder()
    .queue("school.alunos")
    .dead_letter(DeadLetterConfig::new().enabled(true)) // gerax.events.dlx / gerax.events.dlq
    .build()?;
```

Quando habilitada, a fila é criada com `x-dead-letter-exchange`/
`x-dead-letter-routing-key`, e DLX/DLQ são declarados e ligados.
Eventos sem handler seguem `on_unknown_event` (`Ack` por padrão;
`Reject` os encaminha à DLQ).

## Topologia padrão

| Recurso     | Nome                    | Durabilidade |
|-------------|-------------------------|--------------|
| exchange    | `gerax.events`          | true         |
| fila        | `gerax.events`          | true         |
| DLX         | `gerax.events.dlx`      | true         |
| DLQ         | `gerax.events.dlq`      | true         |

Exchange garantido: troca para `Direct` via `RabbitMqConfig::builder()`
(`Headers` não é suportado pelo adapter).

## Exemplo completo

```console
cargo run -p gerax-rabbitmq --example aluno_matriculado
```

Cobre publish-subscribe, consumo assíncrono, retry e DLQ em um único
fluxo (`examples/aluno_matriculado.rs`).

## Testes

```console
cargo test -p gerax-rabbitmq
```

Testes de integração exigem um broker e são executados quando
`RABBITMQ_URI` está definida; caso contrário são ignorados:

```console
RABBITMQ_URI=amqp://gerax:gerax@localhost:5672/%2f cargo test -p gerax-rabbitmq
```

Cenários cobertos: conexão, publish com confirm e persistência, roteamento
por `event_type`, consumo tipado, retry sem loop, DLQ, política de eventos
sem handler e preservação de rastreabilidade ponta-a-ponta.

## Observabilidade

A crate já registra logs estruturados (`tracing`) para
`rabbitmq.publish*`, `rabbitmq.consume*`, `rabbitmq.handler_failed`,
`rabbitmq.retry`, `rabbitmq.dead_letter`, `rabbitmq.ack` etc. — sem
payload completo nem credenciais.

[`RabbitMqConnection`]: src/connection.rs
[`RabbitMqEventBus`]: src/bus.rs
[`RabbitMqConsumer`]: src/consumer.rs
[`RabbitMqConfig`]: src/config.rs
[`RabbitConsumerConfig`]: src/config.rs
[`RabbitHandlerRegistry`]: src/consumer.rs
[`RetryPolicy`]: src/config.rs
[`DeadLetterConfig`]: src/config.rs
[`MessageEnvelope`]: src/envelope.rs
[`EventEnvelope`]: ../gerax-eda/src/envelope.rs