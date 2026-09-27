# Skill: Implementar `gerax-eda`

## Objetivo

Implementar a crate Rust `gerax-eda`, integrante do ecossistema Gerax, fornecendo a infraestrutura de **Event-Driven Architecture (EDA)** para publicação, distribuição e processamento de eventos.

A crate deve complementar, e não duplicar, as responsabilidades existentes em:

* `gerax-cqrs`
* `gerax-es`

A arquitetura desejada é:

```text
Command
   ↓
gerax-cqrs
   ↓
CommandHandler
   ↓
Aggregate
   ↓
gerax-es
   ↓
EventStore
   ↓
Event Publisher
   ↓
gerax-eda
   ↓
EventBus
   ├── EventHandler
   ├── Projection
   ├── Integration Consumer
   └── Notification Consumer
```

---

# 1. Princípios arquiteturais

A implementação deve seguir estas regras fundamentais.

### 1.1 `gerax-eda` não implementa CQRS

Não duplicar:

* `Command`
* `CommandHandler`
* `CommandBus`
* `Query`
* `QueryHandler`
* `QueryBus`

Esses conceitos pertencem a `gerax-cqrs`.

---

### 1.2 `gerax-eda` não implementa Event Sourcing

Não duplicar:

* `Aggregate`
* `EventStore`
* `AggregateRepository`
* `StoredEvent`
* `EventStream`
* `Snapshot`
* optimistic concurrency

Esses conceitos pertencem a `gerax-es`.

---

### 1.3 `gerax-eda` é responsável pelo transporte/distribuição de eventos

Responsabilidades:

```text
Event
   ↓
Envelope
   ↓
Publisher
   ↓
EventBus
   ↓
Subscribers
   ↓
EventHandlers
```

A crate deve permitir:

* publicação de eventos;
* assinatura de eventos;
* roteamento por tipo de evento;
* múltiplos consumidores;
* processamento síncrono;
* processamento assíncrono;
* tratamento de erros;
* retry em nível de infraestrutura;
* identificação de eventos;
* correlation ID;
* causation ID;
* metadata;
* extensibilidade para brokers externos.

---

# 2. Fronteira entre as crates

A arquitetura deve ficar clara:

```text
┌─────────────────────────────────────────────┐
│                  gerax-cqrs                 │
│                                             │
│ Command / Query / Bus / Handler             │
└──────────────────────┬──────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────┐
│                   gerax-es                  │
│                                             │
│ Aggregate / Domain Event / EventStore       │
│ Repository / Snapshot / Version             │
└──────────────────────┬──────────────────────┘
                       │
                       ↓
┌─────────────────────────────────────────────┐
│                  gerax-eda                  │
│                                             │
│ EventBus / Publisher / Subscriber           │
│ EventHandler / Envelope / Routing           │
│ Middleware / Retry / Delivery               │
└─────────────────────────────────────────────┘
```

O domínio não deve conhecer o `EventBus`.

O `Aggregate` gera eventos.

O `gerax-es` persiste esses eventos.

O `gerax-eda` distribui eventos.

---

# 3. Dependências

Inicialmente, manter `gerax-eda` independente de brokers externos.

Dependências internas esperadas:

```toml
[dependencies]
gerax-es = { path = "../gerax-es" }
```

Caso seja necessário utilizar tipos de eventos existentes em `gerax-es`, reutilizá-los em vez de redefini-los.

Não introduzir Kafka, RabbitMQ, NATS ou outra infraestrutura externa na implementação inicial.

A arquitetura deve permitir adapters futuros.

---

# 4. Estrutura da crate

Criar uma estrutura semelhante a:

```text
gerax-eda/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── event.rs
│   ├── envelope.rs
│   ├── bus.rs
│   ├── publisher.rs
│   ├── subscriber.rs
│   ├── handler.rs
│   ├── registry.rs
│   ├── context.rs
│   ├── metadata.rs
│   ├── delivery.rs
│   ├── middleware.rs
│   └── adapters/
│       └── mod.rs
└── tests/
```

A estrutura pode ser ajustada durante a implementação, desde que as responsabilidades permaneçam separadas.

---

# 5. Event

Não criar um segundo conceito incompatível com o evento de `gerax-es`.

O agente deve primeiro inspecionar a API atual de `gerax-es`.

Caso `gerax-es` já forneça um trait `Event`, `DomainEvent` ou equivalente, `gerax-eda` deve utilizá-lo.

O conceito necessário pela EDA é:

```rust
pub trait Event: Send + Sync + 'static {
    fn event_type(&self) -> &'static str;
}
```

Caso o trait existente seja mais apropriado, reutilizá-lo.

Não criar dois traits semanticamente equivalentes.

---

# 6. EventEnvelope

Criar um envelope independente do payload do evento.

Conceito:

```rust
pub struct EventEnvelope<E> {
    pub id: EventId,
    pub event: E,
    pub metadata: EventMetadata,
}
```

O envelope deve permitir representar:

```text
Event
 ├── event_id
 ├── event_type
 ├── timestamp
 ├── correlation_id
 ├── causation_id
 └── metadata
```

---

# 7. EventId

Criar um identificador próprio para a mensagem/evento publicado.

Exemplo conceitual:

```rust
pub struct EventId(Uuid);
```

Usar `uuid` de maneira consistente com o restante do workspace.

Não gerar um novo UUID quando o evento já possuir um identificador persistido no `gerax-es`, caso a arquitetura existente já forneça esse identificador.

---

# 8. EventMetadata

Criar metadata extensível.

Exemplo:

```rust
pub struct EventMetadata {
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: Option<CorrelationId>,
    pub causation_id: Option<CausationId>,
    pub headers: HashMap<String, String>,
}
```

Evitar acoplar metadata a HTTP, Kafka, RabbitMQ ou qualquer transporte específico.

---

# 9. EventHandler

Criar a abstração para consumidores de eventos.

Interface conceitual:

```rust
#[async_trait]
pub trait EventHandler<E>: Send + Sync
where
    E: Event,
{
    async fn handle(
        &self,
        event: EventEnvelope<E>,
    ) -> Result<(), EventHandlerError>;
}
```

A assinatura final deve respeitar o padrão assíncrono já utilizado pelo workspace Gerax.

O handler deve conhecer o evento, mas não o mecanismo de transporte.

---

# 10. EventBus

Criar a abstração principal:

```rust
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish<E>(
        &self,
        event: EventEnvelope<E>,
    ) -> Result<(), EventBusError>
    where
        E: Event;
}
```

Entretanto, a implementação final deve considerar a necessidade de **type erasure**, pois o bus precisa armazenar handlers de diferentes tipos de evento.

Não utilizar `Any` indiscriminadamente.

Criar uma abstração explícita para:

```text
Event
   ↓
ErasedEvent
   ↓
HandlerRegistry
```

---

# 11. EventHandlerRegistry

O registry deve permitir registrar handlers por tipo de evento.

Conceito:

```rust
registry.register::<AlunoMatriculado>(
    AtualizarDiario::new(...)
);

registry.register::<AlunoMatriculado>(
    AtualizarFinanceiro::new(...)
);
```

O resultado conceitual:

```text
AlunoMatriculado
       │
       ├── AtualizarDiario
       ├── AtualizarFinanceiro
       └── EnviarNotificacao
```

O registry deve suportar **N handlers para um mesmo evento**.

Não assumir relação 1:1 entre evento e handler.

---

# 12. Event Routing

O roteamento deve ser baseado no tipo lógico do evento.

Exemplo:

```text
event_type = "aluno.matriculado"
```

ou no mecanismo já utilizado pelo Gerax.

Evitar utilizar o nome Rust completo:

```text
meu_crate::domain::AlunoMatriculado
```

como contrato externo.

O nome lógico do evento deve poder permanecer estável mesmo se a implementação Rust mudar.

---

# 13. InMemoryEventBus

A primeira implementação concreta deve ser:

```text
InMemoryEventBus
```

Características:

* sem broker externo;
* registro de múltiplos handlers;
* publicação assíncrona;
* isolamento entre handlers;
* propagação controlada de erros;
* adequada para testes e aplicações monolíticas.

Exemplo:

```rust
let bus = InMemoryEventBus::new();

bus.register::<AlunoMatriculado>(
    AtualizarDiario::new(...)
);

bus.register::<AlunoMatriculado>(
    EnviarNotificacao::new(...)
);

bus.publish(event).await?;
```

---

# 14. Semântica de publicação

Definir explicitamente a semântica inicial.

A implementação deve documentar se o processamento padrão é:

```text
publish
   ↓
handler 1
   ↓
handler 2
   ↓
handler 3
```

ou:

```text
publish
   ↓
┌──────┬──────┬──────┐
↓      ↓      ↓
H1     H2     H3
```

A implementação deve preferir isolamento entre handlers.

Uma falha em um consumidor não deve impedir silenciosamente os demais consumidores de receberem o evento.

A política final de erro deve ser explicitamente documentada.

---

# 15. Delivery

Criar uma abstração para representar entrega:

```rust
pub struct EventDelivery {
    pub event_id: EventId,
    pub attempt: u32,
}
```

Isso permitirá posteriormente implementar:

* retry;
* dead-letter;
* backoff;
* observabilidade;
* idempotência.

Não implementar um sistema distribuído complexo na primeira versão.

---

# 16. Retry

Criar abstrações que permitam retry futuro.

Por exemplo:

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
}
```

A implementação inicial pode ser simples.

Não introduzir complexidade de broker.

---

# 17. Idempotência

A arquitetura deve reconhecer que consumidores de EDA podem receber o mesmo evento mais de uma vez.

Documentar explicitamente:

```text
Event delivery != exactly once
```

O sistema deve favorecer consumidores idempotentes.

Se for implementado um mecanismo de deduplicação, mantê-lo como responsabilidade de infraestrutura e não do domínio.

---

# 18. Middleware

Criar uma abstração opcional para middleware de eventos.

Conceito:

```text
Event
  ↓
Middleware
  ↓
Middleware
  ↓
Handler
```

Casos de uso futuros:

* logging;
* tracing;
* métricas;
* autenticação/autorização de mensagens;
* retry;
* correlation;
* observabilidade.

Não acoplar middleware a HTTP.

---

# 19. Publisher

Separar:

```text
EventPublisher
```

de:

```text
EventBus
```

Conceitualmente:

```text
EventStore
    ↓
EventPublisher
    ↓
EventBus
```

O `EventPublisher` representa a transição entre eventos persistidos e eventos distribuídos.

Isso é importante para suportar futuramente:

```text
EventStore
    ↓
Outbox
    ↓
Publisher
    ↓
Broker
```

---

# 20. EventStore NÃO deve conhecer EventBus

Não criar:

```rust
trait EventStore {
    async fn append_and_publish(...);
}
```

como responsabilidade obrigatória.

Manter separação:

```text
EventStore
    ↓
persistência

EventPublisher
    ↓
publicação
```

A integração pode ser feita em uma camada superior.

---

# 21. Outbox preparado para extensão

A arquitetura deve deixar espaço para:

```text
Aggregate
   ↓
EventStore
   ↓
Outbox
   ↓
Publisher
   ↓
EventBus / Broker
```

Não é obrigatório implementar Outbox na primeira versão.

Porém, as abstrações não devem impedir essa evolução.

---

# 22. Integration Event

Não assumir que todo `DomainEvent` é automaticamente um evento de integração.

Manter conceitualmente:

```text
DomainEvent
     ↓
IntegrationEvent
```

quando necessário.

Exemplo:

```text
Domain:
    AlunoMatriculado

Integration:
    AlunoMatriculadoV1
```

O contrato externo deve ser independente da estrutura interna do domínio.

---

# 23. Adapters futuros

A crate deve permitir adapters:

```text
gerax-eda
    │
    ├── InMemory
    │
    ├── Kafka
    │
    ├── NATS
    │
    ├── RabbitMQ
    │
    └── outros
```

Esses adapters não devem ser dependências obrigatórias da crate core.

Preferir crates separadas futuramente:

```text
gerax-eda
gerax-eda-kafka
gerax-eda-nats
gerax-eda-rabbitmq
```

---

# 24. Dependência arquitetural

A direção desejada é:

```text
gerax-cqrs
      ↓
gerax-es
      ↓
gerax-eda
```

Porém, evitar dependências circulares.

Especialmente:

```text
gerax-es ──X──> gerax-eda
```

não deve ser obrigatório.

O Event Sourcing deve permanecer utilizável sem EDA.

---

# 25. Exemplo completo

Criar um exemplo conceitual semelhante a:

```rust
#[derive(Debug)]
pub struct AlunoMatriculado {
    pub aluno_id: IdAluno,
    pub turma_id: IdTurma,
}
```

Handler:

```rust
struct AtualizarDiario;

impl EventHandler<AlunoMatriculado> for AtualizarDiario {
    async fn handle(
        &self,
        event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        // atualizar projeção
        Ok(())
    }
}
```

Outro handler:

```rust
struct EnviarNotificacao;

impl EventHandler<AlunoMatriculado> for EnviarNotificacao {
    async fn handle(
        &self,
        event: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        // enviar notificação
        Ok(())
    }
}
```

Registro:

```rust
bus.register::<AlunoMatriculado>(
    AtualizarDiario
);

bus.register::<AlunoMatriculado>(
    EnviarNotificacao
);
```

Publicação:

```rust
bus.publish(
    EventEnvelope::new(
        AlunoMatriculado {
            aluno_id,
            turma_id,
        }
    )
).await?;
```

---

# 26. Integração com gerax-es

Demonstrar explicitamente o fluxo:

```text
Aggregate
    ↓
raise(event)
    ↓
pending events
    ↓
AggregateRepository.save()
    ↓
EventStore.append()
    ↓
persisted events
    ↓
EventPublisher
    ↓
EventBus
```

O `gerax-eda` não deve alterar a semântica do Aggregate ou do EventStore.

---

# 27. Integração com CQRS

Demonstrar:

```text
CommandBus
    ↓
CommandHandler
    ↓
Aggregate
    ↓
gerax-es
    ↓
EventStore
    ↓
gerax-eda
    ↓
Projection
    ↓
ReadModel
```

O `gerax-cqrs` continua responsável pela execução do command.

O `gerax-es` continua responsável pelo estado/event sourcing.

O `gerax-eda` continua responsável pela distribuição dos eventos.

---

# 28. Erros

Criar erros específicos:

```rust
pub enum EventBusError {
    HandlerNotFound,
    HandlerFailed(...),
    InvalidEvent(...),
    PublishFailed(...),
}
```

```rust
pub enum EventHandlerError {
    ProcessingFailed(...),
    Retryable(...),
    NonRetryable(...),
}
```

Os erros devem ser extensíveis e utilizar o padrão de erro adotado pelo workspace.

Não transformar automaticamente todos os erros em `Box<dyn Error>`.

---

# 29. Testes obrigatórios

Implementar testes para:

### Registro

```text
register handler
→ handler encontrado
```

### Múltiplos handlers

```text
event
→ handler A
→ handler B
→ handler C
```

### Evento sem handler

```text
event
→ comportamento documentado
```

### Falha de handler

```text
handler A falha
→ comportamento dos handlers B/C permanece conforme política definida
```

### Event type

```text
evento Rust
→ event_type estável
```

### Metadata

Testar:

* event_id;
* timestamp;
* correlation_id;
* causation_id;
* headers.

### Concorrência

Testar publicação concorrente.

### Idempotência

Testar que o framework não assume entrega exactly-once.

---

# 30. Testes de integração

Criar pelo menos um teste:

```text
Command
 ↓
Aggregate
 ↓
EventStore
 ↓
EventPublisher
 ↓
EventBus
 ↓
Projection
```

Esse teste deve demonstrar que as três crates trabalham juntas:

```text
gerax-cqrs
    +
gerax-es
    +
gerax-eda
```

---

# 31. Documentação

Criar documentação em `lib.rs` explicando:

* o que é EDA;
* diferença entre Command e Event;
* diferença entre Domain Event e Integration Event;
* relação entre CQRS, Event Sourcing e EDA;
* EventStore versus EventBus;
* publicação síncrona versus assíncrona;
* delivery semantics;
* idempotência;
* retry;
* arquitetura da crate.

Incluir pelo menos um diagrama:

```text
Command
   ↓
CQRS
   ↓
Aggregate
   ↓
EventStore
   ↓
Publisher
   ↓
EDA
   ↓
Handlers / Projections
```

---

# 32. Critérios de aceitação

A implementação estará concluída quando:

* [ ] `gerax-eda` compilar com o workspace;
* [ ] não duplicar responsabilidades de `gerax-cqrs`;
* [ ] não duplicar responsabilidades de `gerax-es`;
* [ ] possuir `EventBus`;
* [ ] possuir `EventHandler`;
* [ ] possuir registry de handlers;
* [ ] suportar múltiplos handlers por evento;
* [ ] possuir `EventEnvelope`;
* [ ] possuir metadata;
* [ ] possuir identificação de evento;
* [ ] possuir correlation/causation;
* [ ] possuir implementação `InMemoryEventBus`;
* [ ] possuir tratamento de erros;
* [ ] possuir testes unitários;
* [ ] possuir testes de integração com `gerax-es`;
* [ ] possuir exemplo de integração com `gerax-cqrs`;
* [ ] possuir documentação;
* [ ] não exigir Kafka/NATS/RabbitMQ;
* [ ] permitir adapters externos no futuro;
* [ ] não acoplar o domínio ao EventBus;
* [ ] não acoplar `EventStore` ao broker;
* [ ] manter separação entre Domain Event e Integration Event.

---

# 33. Não implementar nesta fase

Não implementar ainda:

* Kafka;
* RabbitMQ;
* NATS;
* Redis Streams;
* Outbox completo;
* Dead Letter Queue;
* Event Mesh;
* clusterização;
* distribuição entre processos;
* exatamente-once delivery;
* garantia global de ordenação;
* schema registry externo.

Esses recursos devem ser considerados extensões futuras.

---

# 34. Resultado arquitetural esperado

Ao final, o ecossistema deverá permitir:

```text
                    GERAX
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
    gerax-cqrs    gerax-es      gerax-eda
        │             │             │
     Commands      EventStore     EventBus
     Queries       Aggregate      Handlers
     Buses         Events         Publisher
        │             │             │
        └─────────────┼─────────────┘
                      │
                      ▼
                   Domain
```

Fluxo completo:

```text
                ┌──────────────┐
                │   Command    │
                └──────┬───────┘
                       ↓
                ┌──────────────┐
                │ CommandBus   │
                └──────┬───────┘
                       ↓
                ┌──────────────┐
                │CommandHandler│
                └──────┬───────┘
                       ↓
                ┌──────────────┐
                │  Aggregate   │
                └──────┬───────┘
                       ↓
                 Domain Events
                       ↓
                ┌──────────────┐
                │  EventStore  │
                └──────┬───────┘
                       ↓
                ┌──────────────┐
                │EventPublisher│
                └──────┬───────┘
                       ↓
                ┌──────────────┐
                │   EventBus   │
                └──────┬───────┘
                       │
          ┌────────────┼────────────┐
          ↓            ↓            ↓
     Projection   Integration   Notification
          ↓
      ReadModel
          ↑
          │
      QueryHandler
          ↑
          │
       QueryBus
```

## Regra arquitetural principal

> **`gerax-cqrs` decide o que fazer.
> `gerax-es` registra o que aconteceu.
> `gerax-eda` distribui o que aconteceu.**

Essa separação deve ser preservada durante toda a implementação.
