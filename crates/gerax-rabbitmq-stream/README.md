# gerax-rabbitmq-stream

Adapter de infraestrutura que transporta eventos do ecossistema **Gerax**
([`gerax-eda`](../gerax-eda)) via **RabbitMQ Streams** — o protocolo de
*append-only log* nativo do RabbitMQ, ideal para telemetria, histórico e
consumo com retenção.

```text
Aplicação ──► gerax-eda (EventEnvelope, EventHandler)
                        │
                        ▼
          gerax-rabbitmq-stream       RabbitMQ (plugin rabbitmq_stream)
          ┌──────────────┐            ┌──────────────────────────────┐
          │ publish      │───────────►│ stream `school-events`       │
          │ subscribe    │◄───────────│ log append-only + offsets    │
          │ offset/recon │            └──────────────────────────────┘
          └──────────────┘
```

O domínio **não** conhece RabbitMQ: `gerax-eda` define eventos, envelopes
e handlers; esta crate entrega e recebe essas estruturas no log do broker.

## Requisitos

- Rust (edition conforme o workspace);
- RabbitMQ 4.x **com o plugin `rabbitmq_stream`** habilitado (a imagem
  `rabbitmq:4-management` já vem com o plugin de management, mas o
  bootstrap abaixo liga o de streams);
- dependências: `gerax-eda`, `rabbitmq-stream-client`, `tokio`, `serde`,
  `serde_json`, `chrono`, `uuid`, `async-trait`, `futures`, `thiserror`,
  `tracing`.

> Esta crate **não** depende de `gerax-rabbitmq`/`lapin`: são dois
> transportes independentes sobre o mesmo broker (AMQP 0-9-1 e streams).

## Broker local

`compose.yaml` na raiz do workspace sobe o RabbitMQ de AMQP
(5562/15672). Para streams, use o compose dedicado, que **não** conflita
com as portas do outro projeto:

```console
docker compose -f compose.stream.yaml up -d       # ou: podman-compose -f compose.stream.yaml up -d
```

- Stream: `localhost:5552` (protocolo de streams, **não** 5672)
- Management UI: http://localhost:15673 (`guest`/`guest`)

O compose publica o stream pela rede com `advertised_host localhost` e
usa `rabbitmq:4-management` (nome completo, exigido pelo Podman) com
`rabbitmq_stream` e `rabbitmq_stream_management` habilitados no boot.

> `guest:guest` e credenciais fixas são **apenas** para desenvolvimento.
> Em produção, host/usuário/senha/TLS vêm de configuração segura.

## Bootstrap

[`RabbitMqStreamConnection`] encapsula o `Environment` do cliente
oficial e garante o stream (idempotente):

```rust,no_run
use gerax_rabbitmq_stream::{
    RabbitMqStreamConfig, RabbitMqStreamConnection, StreamConfig,
};

let config = RabbitMqStreamConfig::builder()
    .host("localhost")
    .port(5552) // protocolo de streams, não 5672 (AMQP)
    .build()?;

let connection = RabbitMqStreamConnection::connect(
    config,
    StreamConfig::new("school-events")
        .with_max_length_bytes(1_073_741_824) // 1 GiB de retenção
        .with_max_age(std::time::Duration::from_secs(86_400)),
)
.await?;
```

`ensure_stream()` é chamado no boot de cada producer/consumer: se o stream
não existe, ele é criado; se já existe com parâmetros compatíveis, o
adapter segue; divergência retorna erro explícito em vez de falhar em
silêncio no meio do consumo.

## Publish

O [`RabbitMqStreamEventBus`] embrulha um [`RabbitMqStreamProducer`] e
mantém as subscriptions abertas para shutdown ordenado:

```rust,no_run
use gerax_rabbitmq_stream::{
    PublishMode, RabbitMqStreamEventBusBuilder, StreamConfig,
};

let bus = RabbitMqStreamEventBusBuilder::new(config, StreamConfig::new("school-events"))
    .publish_mode(PublishMode::Confirmed)
    .build()
    .await?;

bus.publish(EventEnvelope::new(AlunoMatriculado { aluno_id: "a-1".into() })).await?;
```

Modos de publicação:

| Modo        | API do cliente                  | Garantia                                  |
|-------------|---------------------------------|-------------------------------------------|
| `Async`     | `send` (padrão)                 | enfileira no buffer interno e retorna     |
| `Confirmed` | `send_with_confirm`             | aguarda confirmação do broker por mensagem |
| `Batch`     | `batch_send_with_confirm`       | confirmação por lote                      |

Para alto volume, `publish_batch` publica um `MessageEnvelope` por item
e confirma o lote inteiro; a confirmação só retorna `Ok` quando **todas**
as mensagens foram confirmadas.

## Formato na wire

O **payload do evento vai no `body`**; a rastreabilidade do
[`gerax_eda::EventEnvelope`] viaja em `application_properties`:

| Propriedade            | Campo                  | Conteúdo                          |
|------------------------|------------------------|-----------------------------------|
| `body`                 | payload serializado    | `EventEnvelope.event` em JSON     |
| `event_type`           | `application_properties` | nome do evento (roteamento)     |
| `event_id`             | `application_properties` | id do envelope                    |
| `version`              | `application_properties` | versão do esquema (1)             |
| `schema_version`       | `application_properties` | versão do schema do payload        |
| `aggregate_type` / `aggregate_id` | `application_properties` | Optional  |
| `correlation_id` / `causation_id` | `application_properties` | ids textuais livres (não UUID)  |
| `occurred_at`          | `application_properties` | timestamp RFC 3339                 |
| `header:*`             | `application_properties` | headers de negócio (`header:x-tenant-id`) |

`correlation_id`/`causation_id` são `Option<String>`: o `gerax-eda` não
impõe UUID, então valores textuais sobrevivem ao round-trip intactos.

Serialização customizada? Implemente [`EventSerializer`] e registre com
`RabbitMqStreamEventBusBuilder::serializer` /
`RabbitMqStreamConsumer::with_serializer` — são genéricos, sem
`dyn`, então zero custo de dynamic dispatch.

## Subscribe e roteamento

O consumidor registra handlers por `event_type` e a subscription entrega
o evento **tipado** ao `EventHandler` do `gerax-eda`:

```rust,no_run
use gerax_rabbitmq_stream::{ConsumerConfig, JsonEventSerializer, StartOffset};

let config = ConsumerConfig::builder()
    .stream("school-events")
    .consumer_name("school-consumer")
    .offset(StartOffset::First)
    .build()?;

let consumer = connection.consumer(config, JsonEventSerializer).await?;
consumer.register::<AlunoMatriculado, _>("aluno.matriculado", handler);
let subscription = consumer.subscribe().await?;
```

- `StartOffset`: `First`, `Last`, `Next` (padrão), `Absolute(u64)` ou
  `Timestamp(DateTime<Utc>)` — traduz 1:1 para o protocolo de streams;
- evento sem handler registrado → log e **avança o offset** (não trava o
  log por evento de outra bounded context);
- payload malformado → log e **avança o offset** (mensagem podre não
  bloqueia o stream);
- o `Drop` da subscription sinaliza shutdown e aborta a task — nenhum
  consumer órfão em runtime current-thread.

## Offset e at-least-once

O offset é gravado **depois** do handler:

| Evento                            | Offset | Efeito                                    |
|-----------------------------------|--------|-------------------------------------------|
| handler `Ok`                      | avança | evento processado                          |
| handler `Err`                     | **fica** | reprocessa no próximo start/reconnect   |
| payload malformado                | avança | podre descartada com log                  |
| evento sem handler                | avança | evento de outra domínio ignorado          |
| processo caiu no meio do handler  | **fica** | redelivery após restart                  |

Isso é **at-least-once**: o handler precisa ser idempotente. O
`event_id` no envelope é a chave natural de deduplicação.

Estratégias de offset:

```rust,no_run
use gerax_rabbitmq_stream::OffsetStrategy;

// padrão: broker persiste por (stream, consumer_name)
let server_side = OffsetStrategy::ServerSide;

// store próprio (o adapter grava; a app persiste onde quiser)
let client_side = OffsetStrategy::client_side(std::sync::Arc::new(MyStore::new()));
```

## Reconexão

O [`ReconnectPolicy`] implementa backoff exponencial **determinístico**
(padrão: `100ms × 2.5^n`, limitado por `max_delay`), sem jitter. Perdas de conexão detectadas no socket são
tratadas como recuperáveis: a subscription reassina, e o offset retoma de
onde parou.

```rust,no_run
use gerax_rabbitmq_stream::ReconnectPolicy;

let policy = ReconnectPolicy::new(
    std::time::Duration::from_millis(100), // primeira espera
    2.0,                                  // fator de crescimento
    std::time::Duration::from_secs(5),    // teto
    0,                                    // 0 = ilimitado
);
// padrão: 100ms → 250ms → 625ms → 1,5s → 3,9s → 5s → 5s … (10 tentativas)
```

Erros de configuração (stream inexistente no resubscribe, consumer sem
nome) são **não recuperáveis**: a subscription termina com erro, em vez
de reconectar para sempre sem possibilidade de shutdown.

## Exemplo completo

```console
cargo run -p gerax-rabbitmq-stream --example aluno_matriculado
RABBITMQ_STREAM_NAME=outras-aulas cargo run -p gerax-rabbitmq-stream --example aluno_matriculado
```

Cobre bootstrap do stream, publish confirmado, consumo tipado com
roteamento por `event_type` e shutdown ordenado
(`examples/aluno_matriculado.rs`).

## Testes

```console
cargo test -p gerax-rabbitmq-stream
```

Os testes de integração exigem broker e rodam quando
`RABBITMQ_STREAM_HOST` está definida; caso contrário são ignorados:

```console
RABBITMQ_STREAM_HOST=localhost RABBITMQ_STREAM_PORT=5552 \
    cargo test -p gerax-rabbitmq-stream
```

Cenários cobertos: conexão, criação idempotente de stream, publish
confirmado/lote, rastreabilidade ponta-a-ponta, consumo tipado por
`event_type`, evento sem handler, payload malformado, offset server-side
e client-side, at-least-once com handler que falha, TLS e limpeza dos
recursos de teste.

O cenário de reconexão derruba e levanta o **broker** (reiniciar o
container em vez de deletar o stream), então é `#[ignore]` por padrão:

```console
RABBITMQ_STREAM_RESTART_CMD="podman restart gerax-rabbitmq-stream" \
    RABBITMQ_STREAM_HOST=localhost \
    cargo test -p gerax-rabbitmq-stream -- --ignored reconnect
```

## Observabilidade

Logs estruturados (`tracing`) nos pontos que importam em produção:

| Evento                                      | Quando                                      |
|---------------------------------------------|---------------------------------------------|
| `rabbitmq stream connection established`    | conexão pronta (campos `endpoint`, `stream`, `tls`) |
| `stream created` / `stream already exists, reusing` | bootstrap idempotente do stream      |
| `producer ready`                            | produtor conectado                          |
| `producer publish confirmed`                | confirmação por mensagem                    |
| `producer.confirmed` / `producer.confirmed batch` | confirmação por mensagem / lote         |
| `producer.publish rejected by the broker`   | recusa do broker                            |
| `consumer connected` / `consumer disconnected` | ciclo de vida da assinatura               |
| `consumer.message_received`                 | entrega recebida                            |
| `consumer.handler_started` / `consumer.handler_completed` | handler ok                          |
| `consumer.handler_failed`                   | handler retornou erro (offset **não** salvo) |
| `malformed message, skipping and advancing offset` | payload ilegível                    |
| `consumer.handler_failed: no handler registered, offset advanced` | evento sem handler |
| `failed to resubscribe, retrying with backoff` | reconexão em andamento                    |
| `resubscribe failed with a non-recoverable error` | erro de configuração                   |
| `reconnect attempts exhausted`              | política de reconexão esgotada              |

Campos estruturados: `stream`, `consumer`, `producer`, `event_type`,
`event_id`, `offset`, `attempt` — **sem** payload completo e **sem**
credenciais. `RUST_LOG=gerax_rabbitmq_stream=debug` mostra o detalhe do
handshake, do offset retomado e das reconexões.

## Limites conhecidos

- **Não** implementa a trait `gerax_eda::EventBus` (ela exige `E: Event +
  Clone`, enquanto a serialização exige `E: Serialize`). Use
  `RabbitMqStreamEventBus` diretamente; `From<RabbitMqStreamError> for
  EventBusError` permite orquestrar `EventStore → EventBus → Stream`
  mantendo os erros no contrato da porta.
- Sem DLQ/retry automático como o adapter de AMQP: em um log, o
  reprocessamento é o **offset**, então retry é Matter: a decisão é do
  handler (NACK deixa o offset parado e o evento volta).
- Replicação multi-nó, super streams e filtros do broker ficam fora do
  adapter; para isso use o reexport `rabbitmq_stream_client` com a API
  de baixo nível.

[`RabbitMqStreamConnection`]: src/connection.rs
[`RabbitMqStreamEventBus`]: src/event_bus.rs
[`RabbitMqStreamProducer`]: src/producer.rs
[`RabbitMqStreamConsumer`]: src/consumer.rs
[`RabbitMqStreamEventBusBuilder`]: src/builder.rs
[`EventSerializer`]: src/message.rs
[`StreamConfig`]: src/config.rs
[`ConsumerConfig`]: src/config.rs
[`RabbitMqStreamConfig`]: src/config.rs
[`ReconnectPolicy`]: src/config.rs
[`PublishMode`]: src/config.rs
[`OffsetStrategy`]: src/offset.rs
[`gerax_eda::EventEnvelope`]: ../gerax-eda/src/envelope.rs
