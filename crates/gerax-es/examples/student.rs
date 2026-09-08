//! Exemplo completo de Event Sourcing com `gerax-es` (Student Aggregate).
//!
//! Fluxo:
//!
//! ```text
//! CreateStudent
//!       │
//!       ▼
//! StudentCreated
//!       │
//!       ▼
//! persist
//!
//!
//! Load Student
//!       │
//!       ▼
//! rehydrate
//!
//!
//! Rename Student
//!       │
//!       ▼
//! StudentRenamed
//!       │
//!       ▼
//! persist
//! ```
//!
//! Executar com:
//!
//! ```bash
//! cargo run -p gerax-es --example student
//! ```

use std::sync::Arc;

use gerax_es::{
    Aggregate, AggregateRepository, EventSourcedRepository, EventStore, InMemoryEventStore,
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

#[derive(Debug, Serialize)]
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(InMemoryEventStore::new());
    let repository = Arc::new(EventSourcedRepository::new(
        store.clone(),
        JsonEventSerializer::<StudentEvent>::new(),
    ));

    // Create Student → StudentCreated → persist
    let mut student = Student::empty("student-1".into());
    student.raise(StudentEvent::StudentCreated { name: "Ana".into() })?;
    repository.save(&mut student).await?;
    println!(
        "created:   version={} name={}",
        student.version().value(),
        student.name
    );

    // Load Student → student reidratado a partir dos eventos persistidos
    let mut reloaded = repository.load(&"student-1".to_string()).await?;
    let pending_count = reloaded.take_events().len();
    println!(
        "reloaded:  version={} name={} (pending={})",
        reloaded.version().value(),
        reloaded.name,
        pending_count
    );

    // Rename Student → StudentRenamed → persist
    reloaded.raise(StudentEvent::StudentRenamed { name: "Bia".into() })?;
    repository.save(&mut reloaded).await?;
    println!(
        "renamed:   version={} name={}",
        reloaded.version().value(),
        reloaded.name
    );

    // Inspect: a stream contém 2 eventos persistidos em ordem
    let persisted = store.load("Student", "student-1").await?;
    println!("stream:    {} evento(s) ->", persisted.len());
    for event in &persisted {
        println!(
            "             v{} {} (payload={})",
            event.version.value(),
            event.event_type,
            event.payload
        );
    }

    // Snapshot opcional: captura o estado em uma versão qualquer
    let snapshot = gerax_es::Snapshot {
        aggregate_type: Student::aggregate_type().to_string(),
        aggregate_id: reloaded.id().clone(),
        version: reloaded.version(),
        state: serde_json::to_value(&reloaded)?,
    };
    println!(
        "snapshot:  {}/{} em v{}",
        snapshot.aggregate_type,
        snapshot.aggregate_id,
        snapshot.version.value()
    );

    Ok(())
}
