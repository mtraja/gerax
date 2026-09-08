//! Snapshots de Aggregates.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::SnapshotStoreError;
use crate::version::Version;

/// Estado de um Aggregate em uma determinada versão.
///
/// Um snapshot permite reconstruir rapidamente um Aggregate a partir do
/// estado salvo (versão `N`) em vez de reprocessar toda a stream desde
/// a origem:
///
/// ```text
/// Sem snapshot:  load 5000 eventos e aplicar todos
/// Com snapshot:  Snapshot(version 4500) + eventos 4501..5000
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Tipo do Aggregate (identidade da stream).
    pub aggregate_type: String,
    /// Identificador do Aggregate.
    pub aggregate_id: String,
    /// Versão em que o estado foi capturado.
    pub version: Version,
    /// Estado do Aggregate serializado.
    pub state: Value,
}

/// Port de armazenamento de snapshots.
///
/// Assim como o [`crate::EventStore`], é uma porta: implementações
/// concretas (memória, PostgreSQL, MongoDB etc.) vivem em adapters.
///
/// Operações:
/// - [`SnapshotStore::load`] — carrega o snapshot mais recente de uma
///   stream, se existir;
/// - [`SnapshotStore::save`] — persiste (ou substitui) um snapshot.
#[async_trait]
pub trait SnapshotStore: Send + Sync {
    /// Carrega o snapshot de uma stream.
    ///
    /// Retorna `None` quando não existir snapshot.
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Option<Snapshot>, SnapshotStoreError>;

    /// Persiste um snapshot da stream.
    async fn save(&self, snapshot: &Snapshot) -> Result<(), SnapshotStoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Implementação mínima de teste para validar o contrato da porta.
    #[derive(Default)]
    struct MockSnapshotStore {
        snapshots: Mutex<HashMap<(String, String), Snapshot>>,
    }

    #[async_trait]
    impl SnapshotStore for MockSnapshotStore {
        async fn load(
            &self,
            aggregate_type: &str,
            aggregate_id: &str,
        ) -> Result<Option<Snapshot>, SnapshotStoreError> {
            let snapshots = self.snapshots.lock().unwrap();
            Ok(snapshots
                .get(&(aggregate_type.to_string(), aggregate_id.to_string()))
                .cloned())
        }

        async fn save(&self, snapshot: &Snapshot) -> Result<(), SnapshotStoreError> {
            let mut snapshots = self.snapshots.lock().unwrap();
            snapshots.insert(
                (
                    snapshot.aggregate_type.clone(),
                    snapshot.aggregate_id.clone(),
                ),
                snapshot.clone(),
            );
            Ok(())
        }
    }

    fn snapshot(version: u64) -> Snapshot {
        Snapshot {
            aggregate_type: "Student".into(),
            aggregate_id: "student-1".into(),
            version: Version::new(version),
            state: serde_json::json!({ "name": "Ana" }),
        }
    }

    #[tokio::test]
    async fn load_sem_snapshot_retorna_none() {
        let store = MockSnapshotStore::default();
        let loaded = store.load("Student", "student-1").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn save_e_load_retornam_o_mesmo_snapshot() {
        let store = MockSnapshotStore::default();
        let snap = snapshot(4500);

        store.save(&snap).await.unwrap();

        let loaded = store.load("Student", "student-1").await.unwrap();
        assert_eq!(loaded, Some(snap));
    }

    #[tokio::test]
    async fn save_substitui_o_snapshot_anterior() {
        let store = MockSnapshotStore::default();
        store.save(&snapshot(4500)).await.unwrap();
        store.save(&snapshot(5000)).await.unwrap();

        let loaded = store.load("Student", "student-1").await.unwrap();
        assert_eq!(loaded.unwrap().version, Version::new(5000));
    }

    #[tokio::test]
    async fn streams_sao_isoladas_por_tipo_e_identificador() {
        let store = MockSnapshotStore::default();
        let mut outro = snapshot(100);
        outro.aggregate_id = "student-2".into();
        store.save(&outro).await.unwrap();

        assert!(store.load("Student", "student-1").await.unwrap().is_none());
        assert!(store.load("Student", "student-2").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn snapshot_serializa_e_desserializa() {
        let snap = snapshot(4500);
        let json = serde_json::to_string(&snap).unwrap();
        let back: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, snap);
    }
}
