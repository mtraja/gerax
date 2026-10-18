//! # gerax-eda
//!
//! Infraestrutura de **Event-Driven Architecture (EDA)** para o
//! ecossistema Gerax: publicação, distribuição e processamento de
//! eventos.
//!
//! ## O que é EDA?
//!
//! *Event-Driven Architecture* é um estilo arquitetural em que o
//! fluxo de controle é determinado por **eventos** — factos já
//! ocorridos — em vez de chamadas diretas entre componentes. Quem
//! publica um evento não conhece (nem precisa conhecer) quem vai
//! consumi-lo: **N consumidores** podem assinar o mesmo evento de
//! forma desacoplada.
//!
//! ## Command versus Event
//!
//! ```text
//! Command                          Event
//! "faça isto"                      "isto aconteceu"
//! pode ser rejeitado               é um fato, imutável
//! tem um único destinatário        tem N destinatários
//! retorno tipado (Output)          sem retorno
//! gerax-cqrs                       gerax-eda
//! ```
//!
//! Um `Command` pede uma mudança; o Aggregate, ao executá-la,
//! produz Domain Events. O `gerax-es` persiste esses eventos; o
//! `gerax-eda` **distribui** o que aconteceu.
//!
//! ## Domain Event versus Integration Event
//!
//! ```text
//! Domain Event                         Integration Event
//! (interno do aggregate)               (contrato entre contextos)
//!        │                                     │
//!   AlunoMatriculado                    AlunoMatriculadoV1
//!   { campos internos }                 { apenas o contrato }
//!        │                                     │
//!        └──────────► quando necessário ───────┘
//! ```
//!
//! Nem todo Domain Event vira evento de integração. Use o marcador
//! [`IntegrationEvent`] quando o contrato público deve permanecer
//! estável independentemente da estrutura interna do domínio (ver
//! [`Event`]).
//!
//! ## CQRS + Event Sourcing + EDA
//!
//! ```text
//!             GERAX
//!               │
//!     ┌─────────┼─────────┐
//!     ▼         ▼         ▼
//! gerax-cqrs  gerax-es  gerax-eda
//!     │         │         │
//!  Commands   EventStore  EventBus
//!  Queries    Aggregate   Handlers
//!  Buses      Events      Publisher
//!     │         │         │
//!     └─────────┼─────────┘
//!               ▼
//!            Domain
//! ```
//!
//! > **`gerax-cqrs` decide o que fazer.
//! > `gerax-es` registra o que aconteceu.
//! > `gerax-eda` distribui o que aconteceu.**
//!
//! ```text
//!            ┌──────────────┐
//!            │   Command    │
//!            └──────┬───────┘
//!                   ↓
//!            ┌──────────────┐
//!            │ CommandBus   │          gerax-cqrs
//!            └──────┬───────┘
//!                   ↓
//!            ┌──────────────┐
//!            │CommandHandler│
//!            └──────┬───────┘
//!                   ↓
//!            ┌──────────────┐
//!            │  Aggregate   │
//!            └──────┬───────┘
//!                   ↓
//!             Domain Events
//!                   ↓
//!            ┌──────────────┐
//!            │  EventStore  │          gerax-es
//!            └──────┬───────┘
//!                   ↓
//!            ┌──────────────┐
//!            │EventPublisher│
//!            └──────┬───────┘
//!                   ↓
//!            ┌──────────────┐
//!            │   EventBus   │          gerax-eda
//!            └──────┬───────┘
//!                   │
//!      ┌────────────┼────────────┐
//!      ↓            ↓            ↓
//!  Projection  Integration  Notification
//!      ↓
//!   ReadModel ← QueryHandler ← QueryBus
//! ```
//!
//! ### Direção da dependência (sem ciclos)
//!
//! ```text
//! gerax-cqrs → gerax-es → gerax-eda
//!
//! gerax-es ──X──> gerax-eda     (Event Sourcing funciona sem EDA)
//! ```
//!
//! Esta crate depende de `gerax-es` (reutiliza [`gerax_es::EventId`],
//! [`gerax_es::StoredEvent`], [`gerax_es::EventPublisher`],
//! [`gerax_es::EventSerializer`]) e **não** duplica responsabilidades:
//!
//! | Responsabilidade | Crate |
//! |---|---|
//! | `Command` / `Query` / Buses / Handlers | `gerax-cqrs` |
//! | `Aggregate` / `EventStore` / snapshots / concorrência | `gerax-es` |
//! | `EventBus` / `EventHandler` / envelope / roteamento | `gerax-eda` |
//!
//! O domínio **não** conhece o `EventBus`, e o `EventStore` **não**
//! conhece o bus nem broker algum.
//!
//! ## EventStore versus EventBus
//!
//! ```text
//! EventStore                       EventBus
//! "o que aconteceu"                "distribuir o que aconteceu"
//! persiste (append, versão)        entrega (publish, handlers)
//! fonte da verdade                 propagação para consumidores
//! gerax-es                         gerax-eda
//! ```
//!
//! A transição entre os dois é o [`EventBusPublisher`]: publica
//! **após** a persistência confirmada — nunca antes.
//!
//! ## Publicação síncrona versus assíncrona
//!
//! O `publish` desta crate é `async`, mas a **semântica do
//! [`InMemoryEventBus`] é síncrona-sequencial**: os handlers rodam
//! na ordem de registro, um após o outro, dentro da mesma chamada
//! (`await` do `publish` retorna quando todos terminaram). Não há
//! fila, thread pool ou entrega fora da chamada.
//!
//! Isso garante:
//!
//! - ordem determinista por evento;
//! - resultado (`Ok`/`HandlerFailed`) disponível ao publicador;
//! - isolamento de falhas sem perda silenciosa de entregas.
//!
//! Bridges externos (Kafka etc., crates futuras) implementam a porta
//! [`EventBus`] e podem oferecer entrega realmente assíncrona — a
//! porta comporta isso sem alterar handlers.
//!
//! ## Delivery semantics
//!
//! ```text
//! Event delivery ≠ exactly once
//! ```
//!
//! - **at-least-once** por design: o framework **não deduplica**
//!   por `event_id` e **não assume** entrega exatamente uma vez;
//! - publicar o mesmo envelope duas vezes entrega **duas vezes**;
//! - consumidores devem ser **idempotentes**;
//! - deduplicação, quando necessária, é responsabilidade de
//!   infraestrutura — nunca do domínio.
//!
//! A identificação ([`gerax_es::EventId`]) existe justamente para
//! que consumidores *possam* implementar idempotência, não para que
//! o bus a imponha.
//!
//! ## Retry
//!
//! Retry é responsabilidade de **infraestrutura**: a crate expõe
//! [`RetryPolicy`] (tentativas por handler) aplicada pelo
//! [`InMemoryEventBus`]. Apenas [`EventHandlerError::Retryable`] é
//! retentado; o padrão é `max_attempts = 1` (sem retry). Sem
//! backoff exponencial, sem broker, sem dead-letter nesta fase —
//! apenas o ponto de extensão ([`EventDelivery`]).
//!
//! ## Middleware
//!
//! [`EventMiddleware`] observa/intercepta **cada tentativa** de
//! entrega com o [`EventContext`] (identidade, correlation,
//! causation, tentativa) — logging, tracing, métricas, auditoria —
//! sem acoplar o domínio a HTTP ou a qualquer transporte.
//!
//! ## Arquitetura da crate
//!
//! ```text
//! gerax-eda
//! ├── Event                  trait de evento (nome lógico estável)
//! ├── IntegrationEvent       marcador de contrato externo
//! ├── EventEnvelope<E>       id + payload + metadata
//! ├── EventMetadata          event_type, occurred_at, correlation,
//! │                          causation, headers
//! ├── EventBus               porta de publicação
//! ├── EventSubscriber        porta de assinatura (registro)
//! ├── EventHandler<E>        consumidor tipado
//! ├── EventHandlerRegistry   N handlers por evento (TypeId)
//! ├── ErasedEvent            type erasure explícita
//! ├── EventBusPublisher      EventStore → EventBus (ponte)
//! ├── InMemoryEventBus       implementação concreta (sem broker)
//! ├── EventMiddleware        hooks before/after por tentativa
//! ├── RetryPolicy/EventDelivery  retry e identificação de entrega
//! └── adapters/              adapters de broker implementam EventBus
//!                            (crates separadas: RabbitMQ, futuro
//!                            Kafka/NATS)
//! ```
//!
//! ## Exemplo
//!
//! ```rust,no_run
//! use gerax_eda::{
//!     Event, EventEnvelope, EventHandler, EventHandlerError,
//!     EventBus, InMemoryEventBus,
//! };
//! use serde::Serialize;
//!
//! #[derive(Debug, Clone, Serialize)]
//! struct AlunoMatriculado { aluno_id: String }
//!
//! impl Event for AlunoMatriculado {
//!     fn event_type(&self) -> &'static str { "aluno.matriculado" }
//! }
//!
//! struct AtualizarDiario;
//! struct EnviarNotificacao;
//!
//! #[async_trait::async_trait]
//! impl EventHandler<AlunoMatriculado> for AtualizarDiario {
//!     async fn handle(
//!         &self,
//!         _e: EventEnvelope<AlunoMatriculado>,
//!     ) -> Result<(), EventHandlerError> { Ok(()) }
//! }
//!
//! #[async_trait::async_trait]
//! impl EventHandler<AlunoMatriculado> for EnviarNotificacao {
//!     async fn handle(
//!         &self,
//!         _e: EventEnvelope<AlunoMatriculado>,
//!     ) -> Result<(), EventHandlerError> { Ok(()) }
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let bus = InMemoryEventBus::new();
//!
//! bus.register::<AlunoMatriculado, _>(AtualizarDiario);
//! bus.register::<AlunoMatriculado, _>(EnviarNotificacao);
//!
//! bus.publish(EventEnvelope::new(AlunoMatriculado {
//!     aluno_id: "a-1".into(),
//! })).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Exemplo completo com CQRS + Event Sourcing + EDA:
//! `cargo run -p gerax-eda --example aluno_matriculado`.

pub mod adapters;
pub mod bus;
pub mod context;
pub mod delivery;
pub mod envelope;
pub mod erased;
pub mod error;
pub mod event;
pub mod handler;
pub mod in_memory;
pub mod metadata;
pub mod middleware;
pub mod publisher;
pub mod registry;
pub mod subscriber;

pub use bus::EventBus;
pub use context::EventContext;
pub use delivery::{EventDelivery, RetryPolicy};
pub use envelope::EventEnvelope;
pub use erased::{ErasedEvent, ErasedEventHandler};
pub use error::{EventBusError, EventHandlerError, HandlerFailure};
pub use event::{Event, IntegrationEvent};
pub use gerax_es::EventId;
pub use handler::EventHandler;
pub use in_memory::InMemoryEventBus;
pub use metadata::{CausationId, CorrelationId, EventMetadata};
pub use middleware::EventMiddleware;
pub use publisher::EventBusPublisher;
pub use registry::EventHandlerRegistry;
pub use subscriber::EventSubscriber;
