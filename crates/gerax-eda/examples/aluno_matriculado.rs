//! Exemplo completo: CQRS + Event Sourcing + EDA.
//!
//! Fluxo:
//!
//! ```text
//! Command (MatricularAluno)
//!    │
//!    ▼
//! CommandBus                 (gerax-cqrs)
//!    │
//!    ▼
//! MatricularAlunoHandler
//!    │
//!    ▼
//! Aggregate (raise AlunoMatriculado)
//!    │
//!    ▼
//! Repository.save ──► EventStore.append      (gerax-es)
//!    │
//!    ▼ persistidos (StoredEvent)
//! EventBusPublisher.publish                  (gerax-eda)
//!    │
//!    ▼
//! InMemoryEventBus
//!    │
//!    ├── AtualizarDiario       (projeção)
//!    └── EnviarNotificacao     (consumidor de integração)
//! ```
//!
//! Rodar com: `cargo run -p gerax-eda --example aluno_matriculado`.

use std::sync::{Arc, Mutex};

use gerax_cqrs::{Command, CommandBus, CommandHandler, CqrsError, HandlerRegistry, Message};
use gerax_eda::{
    Event, EventBusPublisher, EventEnvelope, EventHandler, EventHandlerError, InMemoryEventBus,
};
use gerax_es::{
    Aggregate, AggregateApplier, AggregateRepository, EventPublisher, EventSourcedRepository,
    EventStore, InMemoryEventStore, JsonEventSerializer, Version,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Domínio
// ---------------------------------------------------------------------------

/// Erro de regra de negócio da Matrícula.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("matrícula inválida")]
struct MatriculaError;

/// Evento de domínio retido na persistence (struct com tag interna `event_type`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type")]
struct AlunoMatriculado {
    aluno_id: String,
    turma_id: String,
}

impl Event for AlunoMatriculado {
    fn event_type(&self) -> &'static str {
        "AlunoMatriculado"
    }
}

/// Aggregate `Matricula` (Event Sourcing via gerax-es).
#[derive(Debug, Aggregate)]
struct Matricula {
    id: String,
    turma_id: String,
    version: Version,
    pending: Vec<AlunoMatriculado>,
}

impl AggregateApplier for Matricula {
    type Event = AlunoMatriculado;
    type Error = MatriculaError;

    fn apply(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        self.turma_id = event.turma_id.clone();
        self.version = self.version.next();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CQRS (gerax-cqrs)
// ---------------------------------------------------------------------------

/// Command que inicia a matrícula.
struct MatricularAluno {
    id: String,
    aluno_id: String,
    turma_id: String,
}

impl Message for MatricularAluno {
    type Output = Matricula;
}

impl Command for MatricularAluno {}

/// Tipo concreto da ponte `EventStore → EventBus`.
type Publisher = EventBusPublisher<InMemoryEventBus, JsonEventSerializer<AlunoMatriculado>>;

/// CommandHandler → executa no domínio → persiste no Event Store → publica.
#[derive(Clone)]
struct MatricularAlunoHandler {
    repository: Arc<
        EventSourcedRepository<
            Matricula,
            Arc<InMemoryEventStore>,
            JsonEventSerializer<AlunoMatriculado>,
        >,
    >,
    store: Arc<InMemoryEventStore>,
    publisher: Arc<Publisher>,
}

#[async_trait::async_trait]
impl CommandHandler for MatricularAlunoHandler {
    type Command = MatricularAluno;

    async fn handle(&self, command: MatricularAluno) -> Result<Matricula, CqrsError> {
        let mut matricula = Matricula::empty(command.id);

        matricula
            .raise(AlunoMatriculado {
                aluno_id: command.aluno_id,
                turma_id: command.turma_id,
            })
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        // 1. persistência confirmada (sempre antes de publicar)
        self.repository
            .save(&mut matricula)
            .await
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        // 2. eventos persistidos → distribuição (gerax-eda)
        let persisted = self
            .store
            .load(Matricula::aggregate_type(), matricula.id())
            .await
            .map_err(|e| CqrsError::Execution(e.to_string()))?;
        self.publisher
            .publish(&persisted)
            .await
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        Ok(matricula)
    }
}

// ---------------------------------------------------------------------------
// Consumidores EDA (gerax-eda)
// ---------------------------------------------------------------------------

/// Projeção: atualiza um "diário" (read model em memória).
#[derive(Clone, Default)]
struct AtualizarDiario {
    diario: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl EventHandler<AlunoMatriculado> for AtualizarDiario {
    async fn handle(
        &self,
        envelope: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        self.diario.lock().unwrap().push(format!(
            "aluno {} matriculado na turma {}",
            envelope.event.aluno_id, envelope.event.turma_id
        ));
        Ok(())
    }
}

/// Consumidor de integração: conta as notificações entregues.
#[derive(Clone, Default)]
struct EnviarNotificacao {
    total: Arc<Mutex<usize>>,
}

#[async_trait::async_trait]
impl EventHandler<AlunoMatriculado> for EnviarNotificacao {
    async fn handle(
        &self,
        _envelope: EventEnvelope<AlunoMatriculado>,
    ) -> Result<(), EventHandlerError> {
        *self.total.lock().unwrap() += 1;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Aplicação
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(InMemoryEventStore::new());
    let repository = Arc::new(EventSourcedRepository::new(
        store.clone(),
        JsonEventSerializer::<AlunoMatriculado>::new(),
    ));

    // Bus com N handlers para o MESMO evento:
    let bus = InMemoryEventBus::new();
    let diario = AtualizarDiario::default();
    let notificacao = EnviarNotificacao::default();
    bus.register::<AlunoMatriculado, _>(diario.clone());
    bus.register::<AlunoMatriculado, _>(notificacao.clone());

    // Ponte EventStore → EventBus
    let publisher = Arc::new(EventBusPublisher::new(
        bus,
        JsonEventSerializer::<AlunoMatriculado>::new(),
    ));

    // CommandBus (gerax-cqrs) com o handler que persiste + publica.
    let mut registry = HandlerRegistry::new();
    registry.register_command(MatricularAlunoHandler {
        repository,
        store: store.clone(),
        publisher,
    })?;
    let command_bus = CommandBus::new(Arc::new(registry));

    let matricula = command_bus
        .dispatch(MatricularAluno {
            id: "m-1".into(),
            aluno_id: "a-1".into(),
            turma_id: "t-2024".into(),
        })
        .await?;
    assert_eq!(matricula.turma_id, "t-2024");

    assert_eq!(
        *diario.diario.lock().unwrap(),
        vec!["aluno a-1 matriculado na turma t-2024".to_string()]
    );
    assert_eq!(*notificacao.total.lock().unwrap(), 1);

    // Stream persistida no Event Store (gerax-es) é a fonte da verdade.
    let persisted = store.load(Matricula::aggregate_type(), "m-1").await?;
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].event_type, "AlunoMatriculado");

    println!("diário:       {:?}", *diario.diario.lock().unwrap());
    println!("notificações: {}", *notificacao.total.lock().unwrap());
    println!("eventos no EventStore: {}", persisted.len());

    Ok(())
}
