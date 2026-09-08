//! Publicação de eventos após a persistência.

use async_trait::async_trait;

use crate::error::PublishError;
use crate::event::StoredEvent;

/// Publica eventos **já persistidos** para consumidores e projeções.
///
/// ## Ordem obrigatória
///
/// A publicação só deve ocorrer depois da persistência confirmada:
///
/// ```text
/// Aggregate
///     │
///     ▼
/// EventStore.append
///     │
///     ▼ persistência confirmada
/// EventPublisher.publish
///     │
///     ▼
/// Consumers / Projections
/// ```
///
/// Nunca publicar antes de persistir:
///
/// ```text
/// (errado) publish → persist
/// ```
///
/// ## Outbox (extensão futura)
///
/// O núcleo de `gerax-es` **não** implementa Outbox. Porém, a API é
/// projetada para que adapters transacionais (PostgreSQL, MongoDB)
/// publiquem no mesmo limite da persistência:
///
/// ```text
/// Aggregate
///    │
///    ▼
/// Transaction
///    ├── EventStore      (append)
///    └── Outbox          (mensagem a publicar)
///        │
///        ▼ (commit)
///    Event Bus / Consumidores
/// ```
///
/// O ponto de extensão é a implementação concreta do
/// [`EventStore`](crate::EventStore): ela pode inserir a mensagem no
/// Outbox na mesma transação do append, sem alterar esta API.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publica eventos persistidos para os consumidores.
    ///
    /// Os eventos devem vir de uma fonte já confirmada (ex.: o retorno
    /// de um append no Event Store), nunca de eventos ainda não
    /// persistidos.
    async fn publish(&self, events: &[StoredEvent]) -> Result<(), PublishError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use chrono::Utc;
    use serde::{Deserialize, Serialize};

    use crate::aggregate::Aggregate;
    use crate::event::EventId;
    use crate::event_store::EventStore;
    use crate::memory::InMemoryEventStore;
    use crate::metadata::EventMetadata;
    use crate::repository::{AggregateRepository, EventSourcedRepository};
    use crate::serialization::JsonEventSerializer;
    use crate::version::Version;

    #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
    #[error("noop")]
    struct NoopError;

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
        type Error = NoopError;

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

    #[derive(Default)]
    struct RecordingPublisher {
        events: Mutex<Vec<StoredEvent>>,
    }

    #[async_trait]
    impl EventPublisher for RecordingPublisher {
        async fn publish(&self, events: &[StoredEvent]) -> Result<(), PublishError> {
            self.events.lock().unwrap().extend_from_slice(events);
            Ok(())
        }
    }

    fn stored_created(aggregate_id: &str, version: u64) -> StoredEvent {
        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: aggregate_id.into(),
            aggregate_type: "Student".into(),
            event_type: "StudentCreated".into(),
            version: Version::new(version),
            payload: serde_json::json!({ "name": "Ana" }),
            metadata: EventMetadata::new(),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn publish_entrega_eventos_aos_consumidores() {
        let store = InMemoryEventStore::new();
        let publisher = RecordingPublisher::default();

        store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![
                    stored_created("student-1", 1),
                    StoredEvent {
                        event_id: EventId::new(),
                        aggregate_id: "student-1".into(),
                        aggregate_type: "Student".into(),
                        event_type: "StudentRenamed".into(),
                        version: Version::new(2),
                        payload: serde_json::json!({ "name": "Bia" }),
                        metadata: EventMetadata::new(),
                        occurred_at: Utc::now(),
                    },
                ],
            )
            .await
            .unwrap();

        let persisted = store.load("Student", "student-1").await.unwrap();
        publisher.publish(&persisted).await.unwrap();

        let published = publisher.events.lock().unwrap();
        assert_eq!(*published, persisted);
        assert_eq!(published.len(), 2);
    }

    #[tokio::test]
    async fn fluxo_repository_save_entao_publish_entrega_persistidos() {
        let store = Arc::new(InMemoryEventStore::new());
        let repo =
            EventSourcedRepository::new(store.clone(), JsonEventSerializer::<StudentEvent>::new());
        let publisher = RecordingPublisher::default();

        let mut student = Student::empty("student-1".into());
        student
            .raise(StudentEvent::StudentCreated { name: "Ana".into() })
            .unwrap();

        // 1) persistência confirmada
        repo.save(&mut student).await.unwrap();
        let persisted = store.load("Student", "student-1").await.unwrap();
        assert_eq!(persisted.len(), 1);

        // 2) publicação APÓS a persistência
        publisher.publish(&persisted).await.unwrap();

        let published = publisher.events.lock().unwrap();
        assert_eq!(*published, persisted);
        assert_eq!(published[0].event_type, "StudentCreated");
    }

    #[tokio::test]
    async fn publish_com_lista_vazia_eh_aceito() {
        let publisher = RecordingPublisher::default();
        publisher.publish(&[]).await.unwrap();
        assert!(publisher.events.lock().unwrap().is_empty());
    }
}
