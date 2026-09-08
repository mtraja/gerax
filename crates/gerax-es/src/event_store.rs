//! Port de persistência de eventos.

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::EventStoreError;
use crate::event::StoredEvent;
use crate::version::Version;

/// Port de persistência de eventos de um Aggregate.
///
/// O Event Store é uma **porta**: sua interface é definida aqui, e
/// implementações concretas (em memória, PostgreSQL, MongoDB etc.)
/// vivem em adapters separados.
///
/// Operações mínimas:
/// - [`EventStore::load`] — carrega uma stream de eventos, em ordem;
/// - [`EventStore::append`] — persiste eventos com *optimistic
///   concurrency*.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Carrega todos os eventos de uma stream em ordem de ocorrência.
    ///
    /// A stream é identificada pelo par `(aggregate_type, aggregate_id)`.
    /// Uma stream inexistente retorna uma lista vazia.
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, EventStoreError>;

    /// Persiste novos eventos em uma stream.
    ///
    /// `expected_version` é a versão conhecida do Aggregate **antes** da
    /// geração dos novos eventos. Se a stream já avançou para uma versão
    /// diferente, o append é rejeitado com
    /// [`ConcurrencyError`](crate::error::ConcurrencyError), evitando
    /// sobrescrever eventos concorrentes.
    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: Version,
        events: Vec<StoredEvent>,
    ) -> Result<(), EventStoreError>;
}

/// Permite compartilhar um `EventStore` por referência (`Arc<dyn EventStore>`).
#[async_trait]
impl<T> EventStore for Arc<T>
where
    T: EventStore + ?Sized,
{
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        (**self).load(aggregate_type, aggregate_id).await
    }

    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: Version,
        events: Vec<StoredEvent>,
    ) -> Result<(), EventStoreError> {
        (**self)
            .append(aggregate_type, aggregate_id, expected_version, events)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use chrono::Utc;

    use crate::event::EventId;
    use crate::metadata::EventMetadata;

    /// Implementação mínima de teste para validar o contrato da porta.
    #[derive(Default)]
    struct MockStore {
        stored: Mutex<Vec<StoredEvent>>,
    }

    #[async_trait]
    impl EventStore for MockStore {
        async fn load(
            &self,
            aggregate_type: &str,
            aggregate_id: &str,
        ) -> Result<Vec<StoredEvent>, EventStoreError> {
            let stored = self.stored.lock().unwrap();
            Ok(stored
                .iter()
                .filter(|event| {
                    event.aggregate_type == aggregate_type && event.aggregate_id == aggregate_id
                })
                .cloned()
                .collect())
        }

        async fn append(
            &self,
            _aggregate_type: &str,
            _aggregate_id: &str,
            _expected_version: Version,
            events: Vec<StoredEvent>,
        ) -> Result<(), EventStoreError> {
            let mut stored = self.stored.lock().unwrap();
            stored.extend(events);
            Ok(())
        }
    }

    fn stored_event(aggregate_id: &str, version: u64, event_type: &str) -> StoredEvent {
        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: aggregate_id.into(),
            aggregate_type: "Student".into(),
            event_type: event_type.into(),
            version: Version::new(version),
            payload: serde_json::json!({}),
            metadata: EventMetadata::new(),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn load_retorna_stream_vazia_quando_inexistente() {
        let store = MockStore::default();
        let events = store.load("Student", "student-1").await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn append_persiste_e_load_retorna_em_ordem() {
        let store = MockStore::default();

        store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![
                    stored_event("student-1", 1, "StudentCreated"),
                    stored_event("student-1", 2, "StudentRenamed"),
                ],
            )
            .await
            .unwrap();

        let events = store.load("Student", "student-1").await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "StudentCreated");
        assert_eq!(events[1].event_type, "StudentRenamed");
    }

    #[tokio::test]
    async fn load_filtra_por_tipo_e_identificador_do_aggregate() {
        let store = MockStore::default();
        store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![stored_event("student-1", 1, "StudentCreated")],
            )
            .await
            .unwrap();
        store
            .append(
                "Student",
                "student-2",
                Version::initial(),
                vec![stored_event("student-2", 1, "StudentCreated")],
            )
            .await
            .unwrap();

        assert_eq!(store.load("Student", "student-1").await.unwrap().len(), 1);
        assert_eq!(store.load("Student", "student-2").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn event_store_utilizavel_como_arc_dyn() {
        let store: Arc<dyn EventStore> = Arc::new(MockStore::default());
        let events = store.load("Student", "student-1").await.unwrap();
        assert!(events.is_empty());
    }
}
