//! FASE 10 — Integração conceitual: CommandHandler → gerax-es Repository.
//!
//! Fluxo:
//!
//! ```text
//! Command
//!    │
//!    ▼
//! CommandHandler
//!    │
//!    ▼
//! Repository.load
//!    │
//!    ▼
//! Aggregate
//!    │
//!    ▼
//! Domain Operation
//!    │
//!    ▼
//! raise event
//!    │
//!    ▼
//! Repository.save
//! ```
//!
//! Rodar com: `cargo run -p gerax-cqrs --example command_handler_es --features event-sourcing`

use std::sync::Arc;

use gerax_cqrs::{Command, CommandHandler, CqrsError, Message};
use gerax_es::{
    Aggregate, AggregateRepository, EventSourcedRepository, InMemoryEventStore,
    JsonEventSerializer, Version,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid student operation")]
struct StudentError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type")]
enum StudentEvent {
    StudentCreated { name: String },
    StudentRenamed { name: String },
}

#[derive(Debug)]
struct Student {
    id: String,
    name: String,
    version: Version,
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
            StudentEvent::StudentCreated { name } => self.name = name.clone(),
            StudentEvent::StudentRenamed { name } => self.name = name.clone(),
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

/// `CommandHandler` que usa o Repository de `gerax-es` para reconstruir,
/// operar e persistir o Aggregate.
struct CreateStudentHandler<R> {
    repository: R,
}

#[async_trait::async_trait]
impl<R> gerax_cqrs::CommandHandler for CreateStudentHandler<R>
where
    R: AggregateRepository<Student> + 'static,
{
    type Command = CreateStudent;

    async fn handle(&self, command: CreateStudent) -> Result<Student, CqrsError> {
        let mut student = Student::empty(command.id);

        // Domain Operation: raise event (aplica + acumula em pending)
        student
            .raise(StudentEvent::StudentCreated { name: command.name })
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        // Repository.save: take_events → EventStore.append
        self.repository
            .save(&mut student)
            .await
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        Ok(student)
    }
}

struct RenameStudentHandler<R> {
    repository: R,
}

#[async_trait::async_trait]
impl<R> gerax_cqrs::CommandHandler for RenameStudentHandler<R>
where
    R: AggregateRepository<Student> + 'static,
{
    type Command = RenameStudent;

    async fn handle(&self, command: RenameStudent) -> Result<Student, CqrsError> {
        // Repository.load: eventos do Event Store → reapply → Aggregate
        let mut student = self
            .repository
            .load(&command.id)
            .await
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        student
            .raise(StudentEvent::StudentRenamed { name: command.name })
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        self.repository
            .save(&mut student)
            .await
            .map_err(|e| CqrsError::Execution(e.to_string()))?;

        Ok(student)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(InMemoryEventStore::new());
    let repository = Arc::new(EventSourcedRepository::new(
        store.clone(),
        JsonEventSerializer::<StudentEvent>::new(),
    ));

    let create = CreateStudentHandler {
        repository: repository.clone(),
    };
    let rename = RenameStudentHandler {
        repository: repository.clone(),
    };

    // Create Student → StudentCreated → persist
    let student = create
        .handle(CreateStudent {
            id: "student-1".into(),
            name: "Ana".into(),
        })
        .await?;
    println!("created:  {:?} name={}", student.version(), student.name);

    // Load Student → StudentRenamed → persist
    let student = rename
        .handle(RenameStudent {
            id: "student-1".into(),
            name: "Bia".into(),
        })
        .await?;
    println!("renamed:  {:?} name={}", student.version(), student.name);

    // Load a partir de eventos persistidos
    let reloaded = repository.load(&"student-1".to_string()).await?;
    println!(
        "reloaded: {:?} name={} (2 eventos na stream)",
        reloaded.version(),
        reloaded.name
    );

    Ok(())
}
