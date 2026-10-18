# gerax-nats

Adapter de infraestrutura que transporta eventos do ecossistema **Gerax**
([`gerax-eda`](../gerax-eda), [`gerax-es`](../gerax-es)) via **NATS**
(Core NATS + JetStream).

```text
Aplicação ──► gerax-eda (EventBus, EventHandler)
                        │
                        ▼
                gerax-nats                NATS
                ┌──────────────┐        ┌───────────────────────┐
                │ publish     │───────►│ gerax.<bc>.<agg>.<ev> │─ subject
                │ consume     │◄───────│ GERAX_EVENTS          │─ stream
                │ retry/DLQ   │        │ gerax.dlq.<svc>.<ev>  │─ dead-letter
                └──────────────┘        └───────────────────────┘
```

O domínio **não** conhece NATS: `gerax-eda` define eventos, envelopes e
handlers; esta crate entrega e recebe essas estruturas no broker.

## Requisitos

- Rust (edition conforme o workspace);
- NATS 2.10+ com JetStream para o modo persistente (`nats-server -js`);
- dependências: `gerax-eda`, `gerax-es`, `async-nats`, `tokio`, `serde`,
  `serde_json`, `chrono`, `uuid`, `async-trait`, `thiserror`, `futures`,
  `bytes`, `tracing`.

## Broker local

```console
nats-server -js              # JetStream habilitado (porta 4222)
```

ou, via container:

```console
docker run --rm -p 4222:4222 nats:2.10 -js
```

## Bootstrap

O adapter funciona com uma única [`NatsClient`] reutilizada por
produtores e consumidores ([`NatsEventBus`], [`NatsPublisher`],
[`NatsConsumer`]):

```rust,no_run
use gerax_nats::{NatsClient, NatsConfig};

let config = NatsConfig::builder()
    .url("nats://localhost:4222")
    .name("school-service")
    .build()?;
let client = NatsClient::connect(config).await?;
```

A configuração é explícita e validada no `build()` (URL, autenticação,
prefixo de subjects, stream). Autenticação por token **ou**
usuário/senha — nunca ambos. JetStream é habilitado por padrão; use
`jetstream_disabled()` para Core NATS puro.

## Subjects (política única)

Todo subject é derivado do `subject_prefix` configurado — nunca
strings soltas no código (`§9`):

```text
<prefixo>.<bounded_context>.<aggregate>.<event>     evento
<prefixo>.dlq.<service>.<event>                     dead-letter
```

O [`SubjectBuilder`] valida cada token (sem `*`/`>` literais em nomes de
evento; `>` só como último token em patterns). Produtor, consumer e
DLQ derivam os subjects do **mesmo** prefixo — uma política, um lugar.

## Publish

```rust,no_run
use gerax_eda::EventEnvelope;
use gerax_nats::NatsEventBus;

let bus = NatsEventBus::connect(config).await?;
bus.publish_event(AlunoMatriculado { aluno_id: "a-1".into() }).await?;
// ou preservando rastreabilidade do fluxo:
bus.publish(EventEnvelope::new(evento).with_correlation_id("fluxo-1")).await?;
```

Cada evento vira um [`NatsEnvelope`] (identidade, tipo,
correlation/causation, versão, payload JSON) e é publicado no subject
`<prefixo>.<event_type>`:

- **JetStream** (padrão): cada publicação aguarda o `PubAck` do servidor
  — persistência confirmada, nunca silêncio;
- **Core NATS**: `publish` + `flush` (confirmação local de envio).

`NatsEventBus` implementa a porta [`gerax_eda::EventBus`]: a porta
valida a consistência `metadata.event_type` × `Event::event_type()`
(`InvalidEvent` antes do transporte) e converte falhas em
`EventBusError::PublishFailed`. Os métodos inerentes
(`publish_event`/`publish_envelope`) devolvem o erro tipado
[`NatsError`]. O escape hatch [`NatsPublisher`] cobre cenários de baixo
nível (ex.: outbox).

## Consumo JetStream (feature `jetstream`)

```rust,no_run
use gerax_nats::{NatsConsumer, NatsConsumerConfig};

let config = NatsConsumerConfig::builder()
    .durable_name("school-service-alunos")
    .stream("GERAX_EVENTS")
    .ack_wait(std::time::Duration::from_secs(30))
    .max_deliver(5)
    .dead_letter("school-service")
    .build()?;

let consumer = NatsConsumer::new(&client, config)?;
consumer.consume_event(handler).await?; // loop: fetch → handler → ack
```

- **ACK após processamento** (`§12`): o ACK (double-ACK) só acontece
  depois que o handler retorna `Ok` — nunca antes;
- **redelivery é do JetStream** (`§13`): falha do handler → no-ACK → o
  servidor reentrega após `ack_wait`, até `max_deliver` — **sem** retry
  manual em loop;
- **replay**: `replay_from_start` (padrão) entrega o histórico da
  stream na criação do consumer (`DeliverPolicy::All`); `false` entrega
  só mensagens novas;
- **DLQ / poison** (`§14`): payload indecifrável ou falha na última
  entrega são encaminhados a `<prefixo>.dlq.<service>.<event>` com o
  payload original, erro, tentativa e timestamp ([`DlqMessage`]) —
  nunca descarte silencioso (sem `dead_letter()`, o poison é confirmado
  com log de erro);
- **idempotência** (`§15`): entregas podem se repetir (at-least-once);
  consumidores deduplicam por `event_id` — e o produtor já envia o
  header `Nats-Msg-Id`, com o qual o JetStream deduplica
  republicações dentro da `duplicate_window`.

Múltiplas instâncias do mesmo `durable_name` **distribuem** as
mensagens entre si (competição saudável); [`NatsClient::queue_subscribe`]
cobre o load balancing em Core NATS.

## Streams

[`NatsJetStream`] administra streams com semântica **não-destrutiva**:

- `ensure_stream()` — idempotente; divergência de configuração em stream
  existente vira erro (nunca recria silenciosamente);
- `delete_stream()` — ação explícita, nunca automática.

Padrão: stream `GERAX_EVENTS` capturando `<prefixo>.>` com
armazenamento em arquivo (`StreamStorage::File`; `Memory` para
testes/benchmarks).

## Request/reply

Síncrono e ponta-a-ponta — para consultas (`aluno.existe`), **não**
como command bus (`§17`):

```rust,no_run
let resposta = client.request("gerax.aluno.existe", payload).await?;
```

Com o timeout configurado (`request_timeout`); sem respondedor o erro
vem tipado ([`NatsError::Request`]).

## Compatibilidade e decisões

- **Features**: `default = ["jetstream"]`. Sem a feature, o crate
  compila em Core NATS puro (`NatsClient`, `NatsEventBus`,
  `NatsPublisher`) e `NatsConsumer`/`NatsJetStream` não existem; com a
  configuração sem `jetstream`, o modo é Core NATS mesmo com a feature
  ativa;
- **Orbit.rs**: não adotado na v1 — `async-nats` já entrega o
  ciclo de vida (reconnect, reconnect buffer, TLS) que o Orbit
  abstrairia, e manter uma dependência a menos preserva a porta de
  escape para a API completa do async-nats. Reavaliar se o ecossistema
  Gerax adotar o Orbit como padrão de runtime;
- **Performance**: sem otimizações prematuras (batching de publish,
  pipelines) — medir com o adapter real antes de complexar;
- **gerax-codec**: não reusado deliberadamente — suas dependências
  transitivas são pesadas para um adapter de transporte; o JSON via
  `serde_json` é o mesmo contrato de formato.

## Exemplos e testes

```console
cargo test -p gerax-nats
```

Testes de integração exigem um servidor e são executados quando
`NATS_URL` está definida; caso contrário são ignorados:

```console
NATS_URL=nats://localhost:4222 cargo test -p gerax-nats
```

Cenários cobertos: Core NATS (publish/subscribe com headers, queue
group, request/reply), streams (`ensure`/`delete`), publicação com
`PubAck`, dedup por `Nats-Msg-Id`, consumer durável (ACK, redelivery
até sucesso, DLQ por `max_deliver`, poison indecifrável) e a porta
`EventBus` ponta-a-ponta.

## Observabilidade

A crate registra logs estruturados (`tracing`) para conexão,
publicação (com stream/sequence e modo), consumo, redelivery e
dead-letter — sem payload completo nem credenciais.

## Erros

[`NatsError`] é tipado e categorizado: `Connection`, `Publish`,
`Subscription`, `Request`, `JetStream`, `Serialization`,
`Deserialization`, `Configuration`, `InvalidEvent`, `InvalidSubject`,
`Timeout`, `Handler` — com `From<NatsError> for EventBusError` para a
porta `gerax-eda`.

[`NatsClient`]: src/client.rs
[`NatsEventBus`]: src/event_bus.rs
[`NatsPublisher`]: src/publisher.rs
[`NatsConsumer`]: src/consumer.rs
[`NatsConfig`]: src/config.rs
[`NatsJetStream`]: src/jetstream.rs
[`SubjectBuilder`]: src/subject.rs
[`NatsEnvelope`]: src/envelope.rs
[`DlqMessage`]: src/consumer.rs
[`NatsError`]: src/error.rs
[`EventEnvelope`]: ../gerax-eda/src/envelope.rs
[`gerax_eda::EventBus`]: ../gerax-eda/src/bus.rs
