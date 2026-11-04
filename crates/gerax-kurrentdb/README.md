# gerax-kurrentdb

Adapter [KurrentDB](https://kurrent.io/) (antigo EventStoreDB) para o
ecossistema **Gerax**. Expõe duas funcionalidades **independentes** que
compartilham o mesmo cliente gRPC:

```text
gerax-kurrentdb
├── event_store     implementa gerax_es::EventStore
└── subscription    Persistent Subscriptions (grupos, ACK/NACK, retry, park)
```

O Event Relay **não** pertence a esta crate: ele é orquestrado pela
[`gerax-eda`](../gerax-eda) e publicado pela [`gerax-nats`](../gerax-nats).

## Requisitos

- Rust edition 2024 (versão mínima do workspace);
- KurrentDB acessível por gRPC (`kurrentdb://` ou `esdb://`);
- para o relay: NATS 2.10+ com JetStream (`nats-server -js`).

## Instalação

```toml
[dependencies]
gerax-kurrentdb = { path = "../gerax-kurrentdb" }
```

Features:

```toml
default = ["eda"]
eda = ["dep:gerax-eda"]
```

Com `eda` (padrão), `KurrentDbEventSource` implementa a porta
`gerax_eda::event_relay::EventSource`. Sem ela, a crate oferece apenas o
Event Store e as Persistent Subscriptions.

## Event Store

```rust,no_run
use gerax_es::{EventStore, Version};
use gerax_kurrentdb::KurrentDbEventStore;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let store = KurrentDbEventStore::connect(
    "kurrentdb://admin:changeit@localhost:2113?tls=false",
).await?;

// overlap: carrega o histórico do Aggregate
let historico = store.load("Aluno", "aluno-1").await?;

// append com concorrência otimista
store.append("Aluno", "aluno-1", Version::new(historico.len() as u64), eventos).await?;
# Ok(())
# }
```

### Streams

Cada Aggregate usa uma stream própria:

```text
{aggregate_type}-{aggregate_id}
Aluno-aluno-1
Turma-turma-9
```

### Versões

O `Version` do Gerax é baseado em `1`; o KurrentDB usa revisões baseadas
em `0`:

```text
Gerax Version 0  ↔  StreamState::NoStream
Gerax Version 1  ↔  StreamRevision(0)
Gerax Version n  ↔  StreamRevision(n - 1)
```

A leitura preserva a ordem crescente de revisão. O `append` valida a
sequência localmente e envia o `StreamState` esperado ao servidor. Um
`WrongExpectedVersion` vira `EventStoreError::Concurrency`: o adapter
**não** faz retry automático — a aplicação recarrega o Aggregate e
reexecuta o Command.

### Metadata

O evento nativo do KurrentDB não possui `aggregate_type`, `aggregate_id`
nem o instante original. Esses campos, junto com `EventMetadata`, são
preservados no *custom metadata* JSON. O payload do Domain Event fica no
*data* do evento, com `event_type` e `event_id` nativos.

## Persistent Subscriptions

Criar/atualizar/remover grupos é uma operação administrativa
**explícita**. O consumidor nunca cria grupos silenciosamente.

```rust,no_run
use gerax_kurrentdb::{
    KurrentDbClient, KurrentDbSubscription, SubscriptionAck,
    SubscriptionConfig, SubscriptionManager, SubscriptionStart,
};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = KurrentDbClient::connect(
    "kurrentdb://admin:changeit@localhost:2113?tls=false",
).await?;

let config = SubscriptionConfig::new("Aluno-aluno-1", "projecao-alunos")
    .start_from(SubscriptionStart::Start)
    .max_retry_count(5)
    .message_timeout_secs(30);

let manager = SubscriptionManager::new(client.clone());
manager.ensure(&config).await?; // idempotente

let subscription = KurrentDbSubscription::new(client, config)?;
let mut session = subscription.subscribe().await?;

while let Ok(event) = session.next_event().await {
    // ... processamento idempotente ...
    session.ack(&event, SubscriptionAck::Ack).await?;
}
# Ok(())
# }
```

### Nomes e grupos

- `SubscriptionConfig::new(stream, group)` para uma stream de Aggregate;
- `SubscriptionConfig::all(group)` para o `$all` (com filtros opcionais);
- instâncias com o **mesmo** grupo compartilham trabalho e checkpoint;
- grupos diferentes possuem checkpoints independentes e recebem o mesmo
  evento.

### ACK, retry e park

| Ação | Efeito |
| --- | --- |
| `Ack` | confirma o processamento |
| `Retry` | devolve ao grupo para reentrega |
| `Park` | remove do fluxo ativo (dead-letter do KurrentDB) |
| `Skip` | ignora sem reentrega e sem park |

O `max_retry_count` do grupo limita as reentregas no servidor.
`SubscriptionManager::replay_parked` devolve mensagens parkeadas ao fluxo
quando a falha for corrigida.

### Reconexão

`KurrentDbSubscription::run(handler)` reconecta automaticamente em falhas
transitórias, respeitando `reconnect_delay_ms`. Para controle fino, use
`subscribe()` e `SubscriptionSession` diretamente.

## Filtros no `$all`

Filtros server-side só são válidos para `$all`:

```rust,no_run
use gerax_kurrentdb::{SubscriptionConfig, SubscriptionFilterConfig};

let config = SubscriptionConfig::all("relay")
    .filter(
        SubscriptionFilterConfig::prefixes(["Aluno-", "Turma-"])
            .exclude_system_events(),
    );
```

`start_from = Revision(..)` é exclusivo de streams regulares;
`start_from = Position { .. }` é exclusivo do `$all`.

## Event Relay (gerax-eda + gerax-nats)

O fluxo completo é:

```text
KurrentDB
   │  Persistent Subscription
   ▼
KurrentDbEventSource          (gerax-kurrentdb, feature `eda`)
   │  IntegrationEvent
   ▼
EventRelay                    (gerax-eda)
   │  EventDispatcher → PublisherHandler
   ▼
NatsIntegrationPublisher      (gerax-nats)
   │  JetStream PubAck
   ▼
ACK no KurrentDB              ← somente após a confirmação
```

```rust,no_run
use gerax_eda::event_relay::{
    EventDispatcher, EventRelay, PublisherHandler, RelayRetryPolicy,
};
use gerax_kurrentdb::{
    KurrentDbClient, KurrentDbEventSource, KurrentDbSubscription,
    SubscriptionConfig, SubscriptionManager,
};
use gerax_nats::{NatsIntegrationPublisher, NatsPublisher};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = KurrentDbClient::connect(
    "kurrentdb://admin:changeit@localhost:2113?tls=false",
).await?;
let config = SubscriptionConfig::all("gerax-event-relay");

SubscriptionManager::new(client.clone()).ensure(&config).await?;

let source = KurrentDbEventSource::new(
    KurrentDbSubscription::new(client, config)?,
);

// `nats_client` já conectado com JetStream habilitado e a stream criada.
let publisher = NatsIntegrationPublisher::new(NatsPublisher::new(&nats_client));
let dispatcher = EventDispatcher::new()
    .register("aluno.matriculado.v1", PublisherHandler::new(publisher));

let mut relay = EventRelay::new(source, dispatcher)
    .with_retry_policy(RelayRetryPolicy::new(5));

relay.run().await?;
# Ok(())
# }
```

### Regras de confirmação

- `dispatch` retorna `Ok` → `Ack` na origem;
- falha **transitória** → `Retry` até `max_retry_count`, depois `Park`;
- falha **permanente** → `Park` (ou `Skip` por política);
- payload inválido vindo do KurrentDB é parkeado automaticamente;
- falha ao publicar **nunca** confirma o evento na origem.

### At-least-once

```text
Não existe exactly-once entre KurrentDB e NATS.
```

Se o processo falhar entre o `PubAck` e o ACK no KurrentDB, o evento será
reentregue e poderá ser republicado. O `event_id` original é preservado e
vira `Nats-Msg-Id`, ativando a deduplicação do JetStream dentro da janela
`duplicate_window`. Isso é proteção adicional — consumidores continuam
responsáveis pela idempotência (ex.: tabela `processed_events`).

## Configuração

```toml
[kurrentdb]
connection_string = "kurrentdb://admin:changeit@localhost:2113?tls=false"

[kurrentdb.subscription]
stream = "$all"
group = "gerax-event-relay"
buffer_size = 100
max_retry_count = 5
start_from = "end"
message_timeout_secs = 30
consumer_strategy = "round_robin"
reconnect_delay_ms = 1000

[kurrentdb.subscription.filter]
prefixes = ["Aluno-", "Turma-"]
exclude_system_events = true
```

```rust,no_run
use gerax_kurrentdb::KurrentDbConfig;

let config = KurrentDbConfig::from_toml_file("config/gerax.toml")?;
config.validate()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`KurrentDbConfig` também aceita JSON (`from_json_str`) e builders
(`KurrentDbConfig::new(connection_string).with_subscription(config)`).
Credenciais devem vir de variáveis de ambiente ou de um gerenciador de
segredos — nunca de arquivos versionados.

## Observabilidade

O adapter usa `tracing` com campos estruturados (`event_id`,
`aggregate_id`, `stream_revision`, `subscription_group`,
`correlation_id`). Payloads sensíveis não são logados.

## Testes

Testes unitários rodam sem infraestrutura:

```sh
cargo test -p gerax-kurrentdb
```

Testes de integração exigem os serviços e são ignorados caso as variáveis
não estejam definidas:

```sh
KURRENTDB_URL='kurrentdb://admin:changeit@localhost:2113?tls=false' \
    cargo test -p gerax-kurrentdb --test event_store --test subscription

KURRENTDB_URL='kurrentdb://admin:changeit@localhost:2113?tls=false' \
NATS_URL='nats://localhost:4222' \
    cargo test -p gerax-kurrentdb --test relay
```

## Exemplos

```sh
cargo run -p gerax-kurrentdb --example event_store
cargo run -p gerax-kurrentdb --example subscription
cargo run -p gerax-kurrentdb --example relay
```

## Decisões arquiteturais

- `gerax-kurrentdb` contém **dois módulos independentes** (`event_store` e
  `subscription`), compartilhando apenas o cliente;
- nenhum acoplamento com NATS;
- nenhum dual-write: o KurrentDB é a fonte da verdade do Event Sourcing;
- o Event Relay vive na `gerax-eda`;
- a publicação/confirmação JetStream vive na `gerax-nats`;
- sem promessa de exactly-once.
