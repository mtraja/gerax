//! Teste de integração: `gerax-cqrs` + `gerax-es` + `gerax-eda`.
//!
//! Fluxo coberto:
//!
//! ```text
//! Command ─► CommandBus ─► CommandHandler ─► Aggregate
//!                    │
//!                    ▼
//!      AggregateRepository.save ─► EventStore.append
//!                    │
//!                    ▼ persistidos
//!      EventStore.load (stream completa)
//!                    │
//!                    ▼
//!      EventBusPublisher.publish ─► InMemoryEventBus
//!                    │
//!                    ├── ProjetarEstudante     (projeção → read model)
//!                    └── ContarEntrega         (2º handler / at-least-once)
//!                    │
//!                    ▼
//!      QueryBus.execute (read model) ─► Output
//! ```
//!
//! A semântica de entrega é *at-least-once*: como `store.load` devolve a
//! stream completa, cada save republica todo o histórico. A projeção é
//! idempotente (aplica os eventos como um conjunto), então o read model
//! converge mesmo com redelivery — e o consumidor de notificação observa
//! as entregas duplicadas.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gerax_cqrs::{
    Command, CommandBus, CommandHandler, CqrsError, HandlerRegistry, Message, Query, QueryBus,
    QueryHandler,
};
use gerax_eda::{
    Event, EventBusPublisher, EventEnvelope, EventHandler, EventHandlerError, InMemoryEventBus,
};
use gerax_es::{
    Aggregate, AggregateRepository, EventPublisher, EventSourcedRepository, EventStore,
    InMemoryEventStore, JsonEventSerializer, Version,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Domínio / Event Sourcing (gerax-es)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
enum StudentError {
    #[error("student already created")]
    AlreadyCreated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type")]
enum StudentEvent {
    StudentCreated { id: String, name: String },
    StudentRenamed { id: String, name: String },
}

impl Event for StudentEvent {
    fn event_type(&self) -> &'static str {
        match self {
            Self::StudentCreated { .. } => "StudentCreated",
            Self::StudentRenamed { .. } => "StudentRenamed",
        }
    }
}

#[derive(Debug)]
struct Student {
    id: String,
    name: String,
    version: Version,
    created: bool,
    pending: Vec<StudentEvent>,
}

impl Aggregate for Student {
    type Id = String;
    type Event = StudentEvent;
    type Error = StudentError;

    fn aggregate_type() -> &'static str {
        "Student"
    }

    fn empty(id: Self::Id) -> Self {
        Self {
            id,
            name: String::new(),
            version: Version::initial(),
            created: false,
            pending: Vec::new(),
        }
    }

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn version(&self) -> Version {
        self.version
    }

    fn apply(&mut self, event: &Self::Event) -> Result<(), Self::Error> {
        match event {
            StudentEvent::StudentCreated { name, .. } => {
                if self.created {
                    return Err(StudentError::AlreadyCreated);
                }
                self.created = true;
                self.name = name.clone();
            }
            StudentEvent::StudentRenamed { name, .. } => {
                self.name = name.clone();
            }
        }
        self.version = self.version.next();
        Ok(())
    }

    fn raise(&mut self, event: Self::Event) -> Result<(), Self::Error> {
        self.apply(&event)?;
        self.pending.push(event);
        Ok(())
    }

    fn take_events(&mut self) -> Vec<Self::Event> {
        std::mem::take(&mut self.pending)
    }
}

// ---------------------------------------------------------------------------
// CQRS (gerax-cqrs)
// ---------------------------------------------------------------------------

type Repo =
    EventSourcedRepository<Student, Arc<InMemoryEventStore>, JsonEventSerializer<StudentEvent>>;
type Publisher = EventBusPublisher<InMemoryEventBus, JsonEventSerializer<StudentEvent>>;

struct CreateStudent {
    id: String,
    name: String,
}

impl Message for CreateStudent {
    type Output = Student;
}

impl Command for CreateStudent {}

struct RenameStudent {
    id: String,
    name: String,
}

impl Message for RenameStudent {
    type Output = Student;
}

impl Command for RenameStudent {}

/// Infraestrutura compartilhada pelos CommandHandlers.
struct Dependencies {
    repository: Repo,
    store: Arc<InMemoryEventStore>,
    publisher: Arc<Publisher>,
}

impl Dependencies {
    /// Persiste → recarrega a stream completa → publica no bus.
    async fn save_and_publish(&self, student: &mut Student) -> Result<(), String> {
        self.repository
            .save(student)
            .await
            .map_err(|e| e.to_string())?;

        let persisted = self
            .store
            .load(Student::aggregate_type(), student.id())
            .await
            .map_err(|e| e.to_string())?;

        self.publisher
            .publish(&persisted)
            .await
            .map_err(|e| e.to_string())
    }
}

struct CreateStudentHandler {
    deps: Dependencies,
}

#[async_trait::async_trait]
impl CommandHandler for CreateStudentHandler {
    type Command = CreateStudent;

    async fn handle(&self, command: CreateStudent) -> Result<Student, CqrsError> {
        let mut student = Student::empty(command.id.clone());
        student
            .raise(StudentEvent::StudentCreated {
                id: command.id.clone(),
                name: command.name,
            })
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        self.deps
            .save_and_publish(&mut student)
            .await
            .map_err(CqrsError::Execution)?;

        Ok(student)
    }
}

struct RenameStudentHandler {
    deps: Dependencies,
}

#[async_trait::async_trait]
impl CommandHandler for RenameStudentHandler {
    type Command = RenameStudent;

    async fn handle(&self, command: RenameStudent) -> Result<Student, CqrsError> {
        let mut student = self
            .deps
            .repository
            .load(&command.id)
            .await
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        student
            .raise(StudentEvent::StudentRenamed {
                id: command.id.clone(),
                name: command.name,
            })
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        self.deps
            .save_and_publish(&mut student)
            .await
            .map_err(CqrsError::Execution)?;

        Ok(student)
    }
}

// ---------------------------------------------------------------------------
// Projeção / Read Model + Query (gerax-eda + gerax-cqrs)
// ---------------------------------------------------------------------------

type ReadModel = Arc<Mutex<HashMap<String, String>>>;

/// Projeção idempotente: `id → nome`.
#[derive(Clone)]
struct ProjetarEstudante {
    alunos: ReadModel,
}

#[async_trait::async_trait]
impl EventHandler<StudentEvent> for ProjetarEstudante {
    async fn handle(&self, envelope: EventEnvelope<StudentEvent>) -> Result<(), EventHandlerError> {
        match envelope.event {
            StudentEvent::StudentCreated { id, name }
            | StudentEvent::StudentRenamed { id, name } => {
                self.alunos.lock().unwrap().insert(id, name);
            }
        }
        Ok(())
    }
}

/// Consumidor de integração: conta entregas (evidencia at-least-once).
#[derive(Clone)]
struct ContarEntrega {
    total: Arc<Mutex<usize>>,
}

#[async_trait::async_trait]
impl EventHandler<StudentEvent> for ContarEntrega {
    async fn handle(
        &self,
        _envelope: EventEnvelope<StudentEvent>,
    ) -> Result<(), EventHandlerError> {
        *self.total.lock().unwrap() += 1;
        Ok(())
    }
}

struct GetStudentName {
    id: String,
}

impl Message for GetStudentName {
    type Output = Option<String>;
}

impl Query for GetStudentName {}

struct GetStudentNameHandler {
    alunos: ReadModel,
}

#[async_trait::async_trait]
impl QueryHandler for GetStudentNameHandler {
    type Query = GetStudentName;

    async fn handle(&self, query: GetStudentName) -> Result<Option<String>, CqrsError> {
        Ok(self.alunos.lock().unwrap().get(&query.id).cloned())
    }
}

// ---------------------------------------------------------------------------
// Teste
// ---------------------------------------------------------------------------

#[tokio::test]
async fn comando_agregado_store_bus_projecao_query() {
    let store = Arc::new(InMemoryEventStore::new());
    let deps = Dependencies {
        repository: EventSourcedRepository::new(
            store.clone(),
            JsonEventSerializer::<StudentEvent>::new(),
        ),
        store: store.clone(),
        publisher: Arc::new(EventBusPublisher::new(
            InMemoryEventBus::new(),
            JsonEventSerializer::<StudentEvent>::new(),
        )),
    };

    // Read model + N handlers no bus (projeção + consumidor).
    let alunos: ReadModel = Arc::new(Mutex::new(HashMap::new()));
    let total = Arc::new(Mutex::new(0usize));

    deps.publisher
        .bus()
        .register::<StudentEvent, _>(ProjetarEstudante {
            alunos: alunos.clone(),
        });
    deps.publisher
        .bus()
        .register::<StudentEvent, _>(ContarEntrega {
            total: total.clone(),
        });

    // Registro CQRS (compartilham do mesmo registry).
    let mut registry = HandlerRegistry::new();
    registry
        .register_command(CreateStudentHandler {
            deps: deps_for(&deps),
        })
        .unwrap();
    registry
        .register_command(RenameStudentHandler {
            deps: deps_for(&deps),
        })
        .unwrap();
    registry
        .register_query(GetStudentNameHandler {
            alunos: alunos.clone(),
        })
        .unwrap();

    let registry = Arc::new(registry);
    let command_bus = CommandBus::new(registry.clone());
    let query_bus = QueryBus::new(registry.clone());

    // 1. create "Ana": publica a stream [StudentCreated].
    let criado = command_bus
        .dispatch(CreateStudent {
            id: "student-1".into(),
            name: "Ana".into(),
        })
        .await
        .unwrap();
    assert_eq!(criado.name, "Ana");

    let name = query_bus
        .execute(GetStudentName {
            id: "student-1".into(),
        })
        .await
        .unwrap();
    assert_eq!(name, Some("Ana".to_string()));

    // 2. rename "Bia": recarrega via repository, publica a stream
    //    [StudentCreated, StudentRenamed] (redelivery de eventos antigos).
    let renomeado = command_bus
        .dispatch(RenameStudent {
            id: "student-1".into(),
            name: "Bia".into(),
        })
        .await
        .unwrap();
    assert_eq!(renomeado.name, "Bia");

    let name = query_bus
        .execute(GetStudentName {
            id: "student-1".into(),
        })
        .await
        .unwrap();
    assert_eq!(name, Some("Bia".to_string()));

    // 3. Event Store é a fonte da verdade: 2 eventos persistidos.
    let persisted = store
        .load(Student::aggregate_type(), "student-1")
        .await
        .unwrap();
    let tipos: Vec<_> = persisted.iter().map(|e| e.event_type.as_str()).collect();
    assert_eq!(tipos, vec!["StudentCreated", "StudentRenamed"]);

    // 4. At-least-once: 1 entrega (create) + 2 entregas (rename com
    //    redelivery do histórico) = 3; a projeção idempotente converge.
    assert_eq!(*total.lock().unwrap(), 3);

    // 5. Read model final consistente apesar das redeliveries.
    let alunos = alunos.lock().unwrap();
    assert_eq!(alunos.get("student-1"), Some(&"Bia".to_string()));
    assert_eq!(alunos.len(), 1);
}

fn deps_for(deps: &Dependencies) -> Dependencies {
    Dependencies {
        repository: EventSourcedRepository::new(
            deps.store.clone(),
            JsonEventSerializer::<StudentEvent>::new(),
        ),
        store: deps.store.clone(),
        publisher: deps.publisher.clone(),
    }
}
