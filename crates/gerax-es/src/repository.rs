//! Repository de Aggregates.

use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use crate::aggregate::Aggregate;
use crate::error::RepositoryError;
use crate::event::{EventId, StoredEvent};
use crate::event_store::EventStore;
use crate::metadata::EventMetadata;
use crate::serialization::EventSerializer;
use crate::version::Version;

/// Porta de acesso a Aggregates persistidos via Event Sourcing.
///
/// Responsabilidades:
/// - [`AggregateRepository::load`] — reidrata um Aggregate a partir da
///   sua stream de eventos;
/// - [`AggregateRepository::save`] — persiste os eventos pendentes do
///   Aggregate.
#[async_trait]
pub trait AggregateRepository<A: Aggregate>: Send + Sync {
    /// Carrega e reidrata um Aggregate.
    ///
    /// Retorna [`RepositoryError::NotFound`] quando a stream não existe.
    async fn load(&self, id: &A::Id) -> Result<A, RepositoryError>;

    /// Persiste os eventos pendentes do Aggregate no `EventStore`.
    async fn save(&self, aggregate: &mut A) -> Result<(), RepositoryError>;
}

/// Implementação concreta de [`AggregateRepository`].
///
/// Conecta um [`Aggregate`], um [`EventSerializer`] e um [`EventStore`]:
///
/// ```text
/// load: EventStore.load(aggregate_type, id)
///         │
///         ▼
///       deserialize → Aggregate::apply → Aggregate reidratado
///
/// save: Aggregate.take_events()
///         │
///         ▼
///       serialize → EventStore.append(expected_version)
/// ```
///
/// A versão esperada do append é a versão do Aggregate **antes** dos
/// novos eventos, garantindo *optimistic concurrency* e impedindo perda
/// de eventos concorrentes.
pub struct EventSourcedRepository<A, ES, S> {
    store: ES,
    serializer: S,
    _marker: PhantomData<fn() -> A>,
}

impl<A, ES, S> EventSourcedRepository<A, ES, S> {
    /// Cria um repository a partir de um `EventStore` e um serializer.
    pub fn new(store: ES, serializer: S) -> Self {
        Self {
            store,
            serializer,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<A, ES, S> AggregateRepository<A> for EventSourcedRepository<A, ES, S>
where
    A: Aggregate + Send,
    A::Id: Clone + ToString + Send + Sync,
    A::Event: Send + Sync,
    A::Error: Send + std::fmt::Display,
    ES: EventStore,
    S: EventSerializer<Event = A::Event>,
    S::Error: std::fmt::Display,
{
    async fn load(&self, id: &A::Id) -> Result<A, RepositoryError> {
        let aggregate_type = A::aggregate_type();
        let aggregate_id = id.to_string();

        let stored = self.store.load(aggregate_type, &aggregate_id).await?;

        if stored.is_empty() {
            return Err(RepositoryError::NotFound(aggregate_id));
        }

        let mut aggregate = A::empty(id.clone());

        for event in stored {
            let domain_event = self
                .serializer
                .deserialize(&event.event_type, &event.payload)
                .map_err(|error| RepositoryError::Serialization(error.to_string()))?;

            aggregate
                .apply(&domain_event)
                .map_err(|error| RepositoryError::Aggregate(error.to_string()))?;
        }

        Ok(aggregate)
    }

    async fn save(&self, aggregate: &mut A) -> Result<(), RepositoryError> {
        let pending = aggregate.take_events();

        if pending.is_empty() {
            return Ok(());
        }

        let pending_count = pending.len() as u64;
        let current = aggregate.version().value();
        let expected = Version::new(current - pending_count);

        let aggregate_type = A::aggregate_type().to_string();
        let aggregate_id = aggregate.id().to_string();

        let mut events = Vec::with_capacity(pending.len());
        let mut next = expected;
        for event in pending {
            next = next.next();

            let serialized = self
                .serializer
                .serialize(&event)
                .map_err(|error| RepositoryError::Serialization(error.to_string()))?;

            events.push(StoredEvent {
                event_id: EventId::new(),
                aggregate_id: aggregate_id.clone(),
                aggregate_type: aggregate_type.clone(),
                event_type: serialized.event_type,
                version: next,
                payload: serialized.payload,
                metadata: EventMetadata::new(),
                occurred_at: Utc::now(),
            });
        }

        self.store
            .append(&aggregate_type, &aggregate_id, expected, events)
            .await?;

        Ok(())
    }
}

/// Permite compartilhar um Repository por referência (`Arc<dyn AggregateRepository<A>>`).
#[async_trait]
impl<A, R> AggregateRepository<A> for Arc<R>
where
    A: Aggregate + Send,
    A::Id: Sync,
    R: AggregateRepository<A> + ?Sized,
{
    async fn load(&self, id: &A::Id) -> Result<A, RepositoryError> {
        (**self).load(id).await
    }

    async fn save(&self, aggregate: &mut A) -> Result<(), RepositoryError> {
        (**self).save(aggregate).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    use crate::error::EventStoreError;
    use crate::memory::InMemoryEventStore;
    use crate::serialization::JsonEventSerializer;

    #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
    enum StudentError {
        #[error("student already created")]
        AlreadyCreated,
    }

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
                StudentEvent::StudentCreated { name } => {
                    if self.created {
                        return Err(StudentError::AlreadyCreated);
                    }
                    self.created = true;
                    self.name = name.clone();
                }
                StudentEvent::StudentRenamed { name } => {
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

    type MemRepo =
        EventSourcedRepository<Student, InMemoryEventStore, JsonEventSerializer<StudentEvent>>;

    fn repo() -> MemRepo {
        EventSourcedRepository::new(
            InMemoryEventStore::new(),
            JsonEventSerializer::<StudentEvent>::new(),
        )
    }

    fn create_student(id: &str, name: &str) -> Student {
        let mut student = Student::empty(id.to_string());
        student
            .raise(StudentEvent::StudentCreated { name: name.into() })
            .unwrap();
        student
    }

    #[tokio::test]
    async fn load_de_aggregate_inexistente_retorna_not_found() {
        let repo = repo();
        let err = repo.load(&"student-1".to_string()).await.unwrap_err();
        assert!(matches!(err, RepositoryError::NotFound(_)));
    }

    #[tokio::test]
    async fn ciclo_create_save_load_reidrata_estado() {
        let repo = repo();
        let mut student = create_student("student-1", "Ana");

        repo.save(&mut student).await.unwrap();

        let mut loaded = repo.load(&"student-1".to_string()).await.unwrap();
        assert_eq!(loaded.name, "Ana");
        assert_eq!(loaded.version(), Version::new(1));
        assert!(loaded.take_events().is_empty());
    }

    #[tokio::test]
    async fn reidratacao_aplica_historico_de_varios_eventos() {
        let repo = repo();
        let mut student = create_student("student-1", "Ana");
        repo.save(&mut student).await.unwrap();

        let mut student = repo.load(&"student-1".to_string()).await.unwrap();
        student
            .raise(StudentEvent::StudentRenamed { name: "Bia".into() })
            .unwrap();
        student
            .raise(StudentEvent::StudentRenamed {
                name: "Cecilia".into(),
            })
            .unwrap();
        repo.save(&mut student).await.unwrap();

        let loaded = repo.load(&"student-1".to_string()).await.unwrap();
        assert_eq!(loaded.name, "Cecilia");
        assert_eq!(loaded.version(), Version::new(3));
    }

    #[tokio::test]
    async fn ciclo_completo_load_modify_save_reload() {
        let repo = repo();
        let mut student = create_student("student-1", "Ana");
        repo.save(&mut student).await.unwrap();

        let mut student = repo.load(&"student-1".to_string()).await.unwrap();
        student
            .raise(StudentEvent::StudentRenamed { name: "Bia".into() })
            .unwrap();
        repo.save(&mut student).await.unwrap();

        let reloaded = repo.load(&"student-1".to_string()).await.unwrap();
        assert_eq!(reloaded.name, "Bia");
        assert_eq!(reloaded.version(), Version::new(2));
    }

    #[tokio::test]
    async fn save_sem_eventos_pendentes_eh_noop() {
        let repo = repo();
        let mut student = Student::empty("student-1".to_string());
        repo.save(&mut student).await.unwrap();

        let err = repo.load(&"student-1".to_string()).await.unwrap_err();
        assert!(matches!(err, RepositoryError::NotFound(_)));
    }

    #[tokio::test]
    async fn save_concorrente_detecta_conflito_de_versao() {
        let repo = repo();
        let mut student = create_student("student-1", "Ana");
        repo.save(&mut student).await.unwrap();

        let mut a = repo.load(&"student-1".to_string()).await.unwrap();
        let mut b = repo.load(&"student-1".to_string()).await.unwrap();

        a.raise(StudentEvent::StudentRenamed { name: "A".into() })
            .unwrap();
        repo.save(&mut a).await.unwrap();

        b.raise(StudentEvent::StudentRenamed { name: "B".into() })
            .unwrap();
        let err = repo.save(&mut b).await.unwrap_err();

        assert!(matches!(
            err,
            RepositoryError::EventStore(EventStoreError::Concurrency(_))
        ));
    }
}
