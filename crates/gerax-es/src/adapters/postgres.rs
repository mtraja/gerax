//! Adapter de [`EventStore`] para PostgreSQL.
//!
//! Usa `sqlx` para persistência em PostgreSQL.
//!
//! ## Schema
//!
//! A tabela é criada automaticamente pelo [`PostgresEventStore::ensure_schema`]:
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS event_store (
//!     event_id UUID PRIMARY KEY,
//!     aggregate_id TEXT NOT NULL,
//!     aggregate_type TEXT NOT NULL,
//!     event_type TEXT NOT NULL,
//!     version BIGINT NOT NULL,
//!     payload JSONB NOT NULL,
//!     metadata JSONB NOT NULL,
//!     occurred_at TIMESTAMPTZ NOT NULL
//! );
//! ```
//!
//! ## Concorrência
//!
//! O adapter usa `optimistic concurrency`: o `append` verifica que a
//! versão atual da stream confere com `expected_version`. Se houver
//! conflito, retorna [`EventStoreError::Concurrency`].
//!
//! ## Uso
//!
//! ```ignore
//! use gerax_es::{EventStore, EventSourcedRepository, JsonEventSerializer};
//! use gerax_es::adapters::postgres::PostgresEventStore;
//!
//! let store = PostgresEventStore::connect("postgres://...").await?;
//! store.ensure_schema().await?;
//! let repo = EventSourcedRepository::new(store, JsonEventSerializer::new());
//! ```

use async_trait::async_trait;
use sqlx::PgPool;

use crate::error::{ConcurrencyError, EventStoreError};
use crate::event::StoredEvent;
use crate::event_store::EventStore;
use crate::version::Version;

/// Adapter de [`EventStore`] para PostgreSQL.
///
/// Cada stream é identificada pelo par `(aggregate_type, aggregate_id)`.
/// Os eventos são mantidos em ordem de `version`.
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    /// Conecta ao PostgreSQL a partir de uma URL.
    pub async fn connect(url: &str) -> Result<Self, EventStoreError> {
        let pool = sqlx::PgPool::connect(url)
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Cria um store a partir de um [`PgPool`] já existente.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retorna referência ao pool subjacente.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Cria a tabela de eventos, se não existir.
    pub async fn ensure_schema(&self) -> Result<(), EventStoreError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS event_store (
                event_id UUID PRIMARY KEY,
                aggregate_id TEXT NOT NULL,
                aggregate_type TEXT NOT NULL,
                event_type TEXT NOT NULL,
                version BIGINT NOT NULL,
                payload JSONB NOT NULL,
                metadata JSONB NOT NULL,
                occurred_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl EventStore for PostgresEventStore {
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        let rows = sqlx::query_as::<_, EventStoreRow>(
            r#"
            SELECT event_id, aggregate_id, aggregate_type, event_type,
                   version, payload, metadata, occurred_at
            FROM event_store
            WHERE aggregate_type = $1 AND aggregate_id = $2
            ORDER BY version ASC
            "#,
        )
        .bind(aggregate_type)
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into_stored()).collect())
    }

    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: Version,
        events: Vec<StoredEvent>,
    ) -> Result<(), EventStoreError> {
        // Determina a versão atual da stream.
        let actual: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM event_store WHERE aggregate_type = $1 AND aggregate_id = $2",
        )
        .bind(aggregate_type)
        .bind(aggregate_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        let actual_version = actual
            .map(|v| Version::new(v.max(0) as u64))
            .unwrap_or(Version::initial());

        if actual_version != expected_version {
            return Err(EventStoreError::Concurrency(ConcurrencyError {
                expected: expected_version,
                actual: actual_version,
            }));
        }

        // Valida a sequência de versões.
        let mut next = expected_version.next();
        for event in &events {
            if event.version != next {
                return Err(EventStoreError::InvalidSequence(format!(
                    "expected version {}, got {}",
                    next,
                    event.version
                )));
            }
            next = next.next();
        }

        // Persiste todos os eventos em uma transação.
        let mut tx = self.pool.begin().await.map_err(|e| EventStoreError::Storage(e.to_string()))?;

        for event in &events {
            let metadata = serde_json::to_value(&event.metadata)
                .map_err(|e| EventStoreError::Storage(e.to_string()))?;
            sqlx::query(
                r#"
                INSERT INTO event_store
                    (event_id, aggregate_id, aggregate_type, event_type, version, payload, metadata, occurred_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(event.event_id.as_uuid())
            .bind(&event.aggregate_id)
            .bind(&event.aggregate_type)
            .bind(&event.event_type)
            .bind(event.version.value() as i64)
            .bind(&event.payload)
            .bind(&metadata)
            .bind(event.occurred_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| EventStoreError::Storage(e.to_string()))?;

        Ok(())
    }
}

/// Linha da tabela event_store mapeada para Rust.
#[derive(sqlx::FromRow)]
struct EventStoreRow {
    event_id: uuid::Uuid,
    aggregate_id: String,
    aggregate_type: String,
    event_type: String,
    version: i64,
    payload: serde_json::Value,
    metadata: serde_json::Value,
    occurred_at: chrono::DateTime<chrono::Utc>,
}

impl EventStoreRow {
    fn into_stored(self) -> StoredEvent {
        let metadata: crate::metadata::EventMetadata = serde_json::from_value(self.metadata)
            .unwrap_or_default();
        StoredEvent {
            event_id: crate::event::EventId::from_uuid(self.event_id),
            aggregate_id: self.aggregate_id,
            aggregate_type: self.aggregate_type,
            event_type: self.event_type,
            version: Version::new(self.version.max(0) as u64),
            payload: self.payload,
            metadata,
            occurred_at: self.occurred_at,
        }
    }
}