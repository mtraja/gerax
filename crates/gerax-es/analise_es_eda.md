# Análise: gerax-es e gerax-eda + adapters

## gerax-es (Event Sourcing)

Principais pontos:
- **Aggregate**: trait `Aggregate` com `AggregateId`, `Event` associado, `version`, `uncommitted_events()`, `apply()`, `new()`. Suporta snapshots via `Snapshot`.
- **EventStore**: porta `EventStore` com `load`, `append`, suporte a concorrência otimista (conflito → `ConcurrencyError`). `InMemoryEventStore` para testes. Adapters como `KurrentDB` e `Postgres` existem.
- **Version**: `gerax_es::Version` (serializa/deserializa). 
- **StoredEvent**: carrega `event_id`, `aggregate_id`, `aggregate_type`, `event_type`, `version`, `payload`, `metadata` (mapa), `occurred_at`.
- **EventPublisher**: porta para transição `persistir → publicar` (implementada por `gerax_eda::EventBusPublisher`).

## gerax-eda (Event-Driven Architecture)

Principais pontos:
- **Event**: trait com `event_type()` estável. `IntegrationEvent` marcador.
- **EventEnvelope<E>**: tem `id`, `aggregate_id: Option<String>`, `version: Option<Version>`, `event`, `metadata` (EventMetadata com correlation/causation/headers). `new()` gera id novo com `aggregate_id/version=None`; `from_stored()` preserva do `StoredEvent`. Builders `with_aggregate`, `with_aggregate_id`, `with_version`.
- **EventBus**: porta com `publish<E: Event + Clone + Serialize>`. Semântica `at-least-once`. `InMemoryEventBus` síncrono-sequencial. 
- **EventBusPublisher**: ponte `gerax-es::EventPublisher` → deserializa via `EventSerializer`, monta com `EventEnvelope::from_stored`.
- **Middleware/Retry**: `EventMiddleware`, `RetryPolicy` (apenas `Retryable`).
- **Erro**: `EventBusError`, `EventHandlerError`.

## Adapters

- **gerax-eda-rabbitmq (AMQP)**: implementa `EventBus`. Publica com `publisher confirm`. Mapeia para AMQP props/headers (`message_id`, `type`, `correlation_id`, `x-causation-id`, `x-<key>`, `x-aggregate-id`, `x-aggregate-version`). Consome via `RabbitMqConsumer` com ack/nack/reject conforme erro. Suporta `publish_new`. Testes passam.
- **gerax-nats**: implementa `EventBus` (Core NATS / JetStream). `NatsEnvelope` com `event_id` em `Nats-Msg-Id` para dedup JetStream. Suporta `from_stored`. Testes passam.
- **gerax-rabbitmq-stream**: corrigido para construção de `EventEnvelope` após adição de `aggregate_id`/`version` (agora preenche a partir do transporte). 
- **gerax-rabbitmq** (outro adapter): também propaga `aggregate_id`/`version` no `MessageEnvelope`.

## Consistências

- **Separação clara**: `gerax-es` persiste, `gerax-eda` distribui, sem ciclos.
- **Identidade**: `EventId` compartilhado. `from_stored` preserva id/aggregate/version/occurred_at.
- **Serialização**: `EventBus` exige `Serialize` (necessário para brokers). Adapters tratam payload como JSON/value.
- **Transporte**: adapters usam seus próprios envelopes (`MessageEnvelope`/`NatsEnvelope`) e mapeiam para/de `EventEnvelope`. `aggregate_id` e `version` já são propagados.
- **Semântica**: `at-least-once`, handlers idempotentes (documentado). `InMemoryEventBus` síncrono-sequencial.

## Observações

- `gerax-eda` agora tem `aggregate_id` e `version` no `EventEnvelope` (adicionados recentemente) - coerentes com `StoredEvent` e com como adapters usam (headers/propriedades). O `EventBusPublisher` já mapeia via `from_stored`. 
- Adapters que constroem `EventEnvelope` a partir de transporte precisam garantir preenchimento correto (rabbitmq-stream corrigido). 
- `gerax-rabbitmq-stream` tinha teste/lib quebrados por campos faltando - corrigido.
- Todos os testes dos pacotes principais (`gerax-eda`, `gerax-eda-rabbitmq`, `gerax-nats`) passam.

Funcionalmente coerentes; mudança de `EventEnvelope` foi propagada para adapters que precisavam.
