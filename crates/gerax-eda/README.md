# gerax-eda

Infraestrutura de **Event-Driven Architecture (EDA)** para o ecossistema Gerax:
publicação, distribuição e processamento de eventos. A crate expõe as portas
(*traits*) do padrão e uma implementação em memória — sem exigir Kafka, NATS,
RabbitMQ ou qualquer broker externo.

## Características

- `Event` — trait de evento com nome lógico estável (`event_type`).
- `#[derive(Event)]` — gera `Event` (e opcionalmente `IntegrationEvent`) para
  structs e enums, com nome/versão configuráveis.
- `IntegrationEvent` — marcador de contrato público, separado do Domain Event.
- `EventEnvelope<E>` — identificação + aggregate/versão + payload + metadata
  do evento.
- `EventMetadata` — `event_type`, `occurred_at`, `correlation_id`,
  `causation_id` e `headers`.
- `EventBus` (publicar) e `EventSubscriber` (assinar) — portas desacopladas.
- `EventHandler<E>` — consumidor tipado; **N handlers por evento**.
- `EventHandlerRegistry` — registro heterogêneo por `TypeId` com type erasure
  explícita (`ErasedEvent`, `ErasedEventHandler`).
- `EventBusPublisher` — ponte de `gerax-es`: publica `StoredEvent` **após** a
  persistência confirmada.
- `InMemoryEventBus` — implementação concreta (sem broker), com isolamento de
  falhas, retry e middleware.
- `RetryPolicy`/`EventDelivery` — retry de infraestrutura (apenas
  `EventHandlerError::Retryable`).
- `EventMiddleware`/`EventContext` — hooks `before`/`after` por tentativa
  (logging, métricas, auditoria).
- Erros tipados com `thiserror` (`EventBusError`, `EventHandlerError`,
  `HandlerFailure`).
- `adapters/` — ponto de extensão para brokers externos no futuro.

## Posição no ecossistema

```text
             GERAX
               │
     ┌─────────┼─────────┐
     ▼         ▼         ▼
gerax-cqrs  gerax-es  gerax-eda
     │         │         │
  Commands   EventStore  EventBus
  Queries    Aggregate   Handlers
  Buses      Events      Publisher
     │         │         │
     └─────────┼─────────┘
               ▼
            Domain
```

> **`gerax-cqrs` decide o que fazer.
> `gerax-es` registra o que aconteceu.
> `gerax-eda` distribui o que aconteceu.**

### Direção da dependência (sem ciclos)

```text
gerax-cqrs → gerax-es → gerax-eda

gerax-es ──X──> gerax-eda     (Event Sourcing funciona sem EDA)
```

Esta crate depende de `gerax-es` (reutiliza `EventId`, `StoredEvent`,
`EventPublisher`, `EventSerializer` do `gerax-es`) e de `gerax-macros` (derive
`Event`), e **não** duplica responsabilidades:

| Responsabilidade | Crate |
|---|---|
| `Command` / `Query` / Buses / Handlers | `gerax-cqrs` |
| `Aggregate` / `EventStore` / snapshots / concorrência | `gerax-es` |
| `EventBus` / `EventHandler` / envelope / roteamento | `gerax-eda` |

O domínio **não** conhece o `EventBus`, e o `EventStore` **não** conhece o bus
nem broker algum.

## Visão geral do fluxo

```text
Domain (Aggregate)            gerax-es
  │  raise events (síncrono)
  ▼
EventStore.append             persistência confirmada
  │
  ▼ StoredEvent
EventBusPublisher             gerax-eda (transição)
  │  deserialize + envelope
  ▼
EventBus                      distribuição
  │
  ├── Projection
  ├── Integration Consumer
  └── Notification Consumer
```

## Módulos

| Módulo | Responsabilidade |
| --- | --- |
| `event` | Trait `Event`, marcador `IntegrationEvent` e derive `Event`. |
| `envelope` | `EventEnvelope<E>` (id + aggregate_id/version + payload + metadata). |
| `metadata` | `EventMetadata`, `CorrelationId`, `CausationId`. |
| `bus` | Porta `EventBus` (`publish`). |
| `subscriber` | Porta `EventSubscriber` (`register`). |
| `handler` | `EventHandler<E>` (também via closures). |
| `erased` | Type erasure: `ErasedEvent`, `ErasedEventHandler`. |
| `registry` | `EventHandlerRegistry` (N handlers por evento via `TypeId`). |
| `publisher` | `EventBusPublisher` (ponte `gerax-es → EventBus`). |
| `delivery` | `EventDelivery` e `RetryPolicy`. |
| `context` | `EventContext` de cada tentativa de entrega. |
| `middleware` | `EventMiddleware` (before/after por tentativa). |
| `in_memory` | `InMemoryEventBus` (implementação concreta). |
| `error` | `EventBusError`, `EventHandlerError`, `HandlerFailure`. |
| `adapters` | Ponto para brokers externos futuros. |

## Exemplo mínimo

```rust,no_run
use gerax_eda::{
    Event, EventBus, EventEnvelope, EventHandler, EventHandlerError,
    InMemoryEventBus,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Event)]
#[event(name = "aluno.matriculado")]
struct AlunoMatriculado { aluno_id: String }

struct AtualizarDiario;
struct EnviarNotificacao;

#[async_trait::async_trait]
impl EventHandler<AlunoMatriculado> for AtualizarDiario {
    async fn handle(
        &self,
        _e: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> { Ok(()) }
}

#[async_trait::async_trait]
impl EventHandler<AlunoMatriculado> for EnviarNotificacao {
    async fn handle(
        &self,
        _e: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> { Ok(()) }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bus = InMemoryEventBus::new();

    // N handlers para o MESMO evento.
    bus.register::<AlunoMatriculado, _>(AtualizarDiario);
    bus.register::<AlunoMatriculado, _>(EnviarNotificacao);

    bus.publish(EventEnvelope::new(AlunoMatriculado {
        aluno_id: "a-1".into(),
    })).await?;
    Ok(())
}
```

## Derive `Event`

O trait `Event` pode ser implementado manualmente ou com o derive reexportado
por esta crate:

```rust
use gerax_eda::Event;

#[derive(Event)]
#[event(name = "aluno.matriculado", version = 1)]
struct AlunoMatriculadoV1 {
    aluno_id: String,
}
```

Regras:

- struct: o nome padrão é o nome do tipo; `name = "..."` sobrescreve e
  `version = N` acrescenta o sufixo `.vN`;
- enum: cada variante vira um `event_type` com o nome da variante; use
  `#[event(name = "...", version = N)]` na variante para sobrescrever;
- `name` não é aceito no nível do enum (cada variante tem seu próprio nome);
- `version` e `integration` no nível do enum são aplicados a todas as
  variantes;
- `integration` no tipo também implementa `IntegrationEvent`.

```rust
use gerax_eda::Event;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Event)]
#[event(integration)]
#[serde(tag = "event_type")]
enum AlunoEvent {
    #[event(name = "aluno.matriculado")]
    Matriculado { aluno_id: String },
    #[event(name = "aluno.cancelado")]
    Cancelado,
}
```

No enum, o padrão de `event_type()` é o nome da variante — o mesmo nome que o
`#[serde(tag = "event_type")]` usa por padrão. Se houver `#[serde(rename)]`, use
`#[event(name = "...")]` para manter os dois lados iguais.

O nome declarado deve ser consistente com a tag `event_type` usada na
persistência (`gerax-es`) e na serialização do broker. Para publicar pelo
`EventBus`, o evento também precisa de `Clone` e `Serialize`:

```rust
#[derive(Debug, Clone, Serialize, Event)]
#[event(name = "aluno.matriculado")]
struct AlunoMatriculado {
    aluno_id: String,
}
```

O derive `Event` gera **apenas** os impls de `Event`/`IntegrationEvent`; os
demais derives continuam explícitos.

## Exemplo completo (CQRS + Event Sourcing + EDA)

Veja [`examples/aluno_matriculado.rs`](examples/aluno_matriculado.rs):

```text
Command ─► CommandBus ─► CommandHandler ─► Aggregate (raise)
                                              │
                                              ▼
                            Repository.save ─► EventStore.append
                                              │
                                              ▼ persistidos
                              EventStore.load (stream)
                                              │
                                              ▼
                         EventBusPublisher.publish ─► InMemoryEventBus
                                                      │
                                                      ├── AtualizarDiario
                                                      └── EnviarNotificacao
```

```sh
cargo run -p gerax-eda --example aluno_matriculado
```

Teste de integração cobrindo o fluxo completo

`Command → Aggregate → EventStore → EventBusPublisher → EventBus → Projeção →
ReadModel → QueryBus`:

```sh
cargo test -p gerax-eda --test cqrs_es_eda
```

## Delivery semantics

```text
Event delivery ≠ exactly once
```

- **at-least-once** por design: o framework **não deduplica** por `event_id`
  e **não assume** entrega exatamente uma vez;
- publicar o mesmo envelope duas vezes entrega **duas vezes**;
- consumidores devem ser **idempotentes**;
- deduplicação, quando necessária, é responsabilidade de infraestrutura —
  nunca do domínio.

A identificação (`gerax_es::EventId` no envelope) existe justamente para que
consumidores *possam* implementar idempotência — não para que o bus a imponha.

## Publicação síncrona versus assíncrona

O `publish` é `async`, mas a semântica do `InMemoryEventBus` é
**síncrona-sequencial**: os handlers rodam na ordem de registro, um após o
outro, dentro da mesma chamada. Não há fila, thread pool ou entrega fora da
chamada. Isso garante ordem determinista por evento, resultado
(`Ok`/`HandlerFailed`) disponível ao publicador e isolamento de falhas sem
perda silenciosa de entregas.

## Retry e Middleware

- `RetryPolicy` configura tentativas por handler; apenas
  `EventHandlerError::Retryable` é retentado; o padrão é `max_attempts = 1`
  (sem retry).
- `EventMiddleware` observa/intercepta **cada tentativa** com o
  `EventContext` (identidade, correlation, causation, tentativa) — logging,
  tracing, métricas — sem acoplar o domínio a transporte algum.

## Testes

```sh
cargo test -p gerax-eda
cargo test -p gerax-eda --test event_macro
cargo clippy -p gerax-eda --all-targets
cargo fmt --package gerax-eda --check
```

## Adapters futuros (fora desta fase)

Não implementado nesta fase: Kafka, RabbitMQ, NATS, Redis Streams, Outbox
completo, Dead Letter Queue, Event Mesh, exatamente-once e ordenação global.
O ponto de extensão é implementar a porta `EventBus` sobre o broker desejado
(ou usar `EventBusPublisher` como ponte a partir do `gerax-es`).