//! Event Store em memória.

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::error::{ConcurrencyError, EventStoreError};
use crate::event::StoredEvent;
use crate::event_store::EventStore;
use crate::version::Version;

/// Implementação em memória de [`EventStore`].
///
/// Utilizada para testes, exemplos e desenvolvimento. Não é persistente
/// entre execuções.
///
/// Estrutura interna: um `HashMap` protegido por `RwLock`. Cada stream é
/// identificada pelo par `(aggregate_type, aggregate_id)` e os eventos
/// são mantidos em ordem de versão.
pub struct InMemoryEventStore {
    streams: RwLock<HashMap<(String, String), Vec<StoredEvent>>>,
}

impl InMemoryEventStore {
    /// Cria um store vazio.
    pub fn new() -> Self {
        Self {
            streams: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventStore for InMemoryEventStore {
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        let streams = self.streams.read().unwrap();
        Ok(streams
            .get(&(aggregate_type.to_string(), aggregate_id.to_string()))
            .cloned()
            .unwrap_or_default())
    }

    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: Version,
        events: Vec<StoredEvent>,
    ) -> Result<(), EventStoreError> {
        let mut streams = self.streams.write().unwrap();
        let key = (aggregate_type.to_string(), aggregate_id.to_string());
        let stream = streams.entry(key).or_default();

        let actual = stream
            .last()
            .map(|event| event.version)
            .unwrap_or(Version::initial());

        if actual != expected_version {
            return Err(EventStoreError::Concurrency(ConcurrencyError {
                expected: expected_version,
                actual,
            }));
        }

        let mut next = expected_version.next();
        for event in &events {
            if event.version != next {
                return Err(EventStoreError::InvalidSequence(format!(
                    "expected version {next}, got {}",
                    event.version
                )));
            }
            next = next.next();
        }

        stream.extend(events);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use chrono::Utc;

    use crate::event::EventId;
    use crate::metadata::EventMetadata;

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
    async fn load_de_stream_inexistente_retorna_vazio() {
        let store = InMemoryEventStore::new();
        let events = store.load("Student", "student-1").await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn append_inicial_persiste() {
        let store = InMemoryEventStore::new();

        store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![stored_event("student-1", 1, "StudentCreated")],
            )
            .await
            .unwrap();

        let events = store.load("Student", "student-1").await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "StudentCreated");
    }

    #[tokio::test]
    async fn append_subsequente_continua_a_stream() {
        let store = InMemoryEventStore::new();
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
                "student-1",
                Version::new(1),
                vec![stored_event("student-1", 2, "StudentRenamed")],
            )
            .await
            .unwrap();

        let events = store.load("Student", "student-1").await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].event_type, "StudentRenamed");
    }

    #[tokio::test]
    async fn load_retorna_eventos_em_ordem() {
        let store = InMemoryEventStore::new();
        store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![
                    stored_event("student-1", 1, "StudentCreated"),
                    stored_event("student-1", 2, "StudentRenamed"),
                    stored_event("student-1", 3, "StudentRenamed"),
                ],
            )
            .await
            .unwrap();

        let events = store.load("Student", "student-1").await.unwrap();
        let types: Vec<_> = events.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(
            types,
            vec!["StudentCreated", "StudentRenamed", "StudentRenamed"]
        );
    }

    #[tokio::test]
    async fn append_com_versao_esperada_incorreta_rejeita() {
        let store = InMemoryEventStore::new();
        store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![stored_event("student-1", 1, "StudentCreated")],
            )
            .await
            .unwrap();

        let err = store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![stored_event("student-1", 2, "StudentRenamed")],
            )
            .await
            .unwrap_err();

        match err {
            EventStoreError::Concurrency(ConcurrencyError { expected, actual }) => {
                assert_eq!(expected, Version::initial());
                assert_eq!(actual, Version::new(1));
            }
            other => panic!("esperava ConcurrencyError, obtido {other:?}"),
        }
    }

    #[tokio::test]
    async fn append_fora_de_ordem_rejeita() {
        let store = InMemoryEventStore::new();

        let err = store
            .append(
                "Student",
                "student-1",
                Version::initial(),
                vec![stored_event("student-1", 2, "StudentRenamed")],
            )
            .await
            .unwrap_err();

        assert!(matches!(err, EventStoreError::InvalidSequence(_)));
    }

    #[tokio::test]
    async fn append_concorrente_vence_apenas_um() {
        let store = Arc::new(InMemoryEventStore::new());

        let mut handles = Vec::new();
        for _ in 0..2 {
            let store = store.clone();
            handles.push(tokio::spawn(async move {
                store
                    .append(
                        "Student",
                        "student-1",
                        Version::initial(),
                        vec![stored_event("student-1", 1, "StudentCreated")],
                    )
                    .await
            }));
        }

        let mut resultados = Vec::new();
        for handle in handles {
            resultados.push(handle.await.unwrap());
        }

        let sucessos = resultados.iter().filter(|r| r.is_ok()).count();
        let conflitos = resultados
            .iter()
            .filter(|r| matches!(r, Err(EventStoreError::Concurrency(_))))
            .count();

        assert_eq!(sucessos, 1);
        assert_eq!(conflitos, 1);
        // stream permanece íntegra com um único evento
        assert_eq!(store.load("Student", "student-1").await.unwrap().len(), 1);
    }
}
