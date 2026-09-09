//! Adapter de [`EventStore`] para MongoDB.
//!
//! Usa o crate `mongodb` para persistência em MongoDB.
//!
//! ## Schema
//!
//! Duas coleções são usadas:
//!
//! - `events`: cada documento representa um evento persistido;
//! - `stream_versions`: controla a versão atual de cada stream,
//!   permitindo `optimistic concurrency` atômico.
//!
//! ## Concorrência
//!
//! O adapter usa `optimistic concurrency`. O `append` executa um
//! `update_one` atômico com filtros que incluem a versão esperada:
//! apenas se a versão atual confere, a atualização é aplicada.
//! Caso contrário, retorna [`EventStoreError::Concurrency`].
//!
//! ## Uso
//!
//! ```ignore
//! use gerax_es::{EventStore, EventSourcedRepository, JsonEventSerializer};
//! use gerax_es::adapters::mongodb::MongoDbEventStore;
//!
//! let store = MongoDbEventStore::connect("mongodb://localhost:27017", "gerax").await?;
//! store.ensure_schema().await?;
//! let repo = EventSourcedRepository::new(store, JsonEventSerializer::new());
//! ```

use async_trait::async_trait;
use bson::{doc, Document};
use futures::StreamExt;
use mongodb::Collection;

use crate::error::{ConcurrencyError, EventStoreError};
use crate::event::StoredEvent;
use crate::event_store::EventStore;
use crate::version::Version;

/// Adapter de [`EventStore`] para MongoDB.
///
/// Cada stream é identificada pelo par `(aggregate_type, aggregate_id)`.
/// Os eventos são mantidos em ordem de `version`.
pub struct MongoDbEventStore {
    events: Collection<Document>,
    versions: Collection<Document>,
}

impl MongoDbEventStore {
    /// Conecta ao MongoDB a partir de uma URI e nome de banco de dados.
    pub async fn connect(uri: &str, database: &str) -> Result<Self, EventStoreError> {
        let options = mongodb::options::ClientOptions::parse(uri)
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;
        let client = mongodb::Client::with_options(options)
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;
        let db = client.database(database);
        Ok(Self {
            events: db.collection("events"),
            versions: db.collection("stream_versions"),
        })
    }

    /// Cria um store a partir de coleções já existentes.
    pub fn new(events: Collection<Document>, versions: Collection<Document>) -> Self {
        Self { events, versions }
    }

    /// Cria os índices necessários.
    pub async fn ensure_schema(&self) -> Result<(), EventStoreError> {
        use mongodb::options::IndexOptions;

        self.events
            .create_index(
                mongodb::IndexModel::builder()
                    .keys(doc! {
                        "aggregate_type": 1,
                        "aggregate_id": 1,
                        "version": 1
                    })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        self.versions
            .create_index(
                mongodb::IndexModel::builder()
                    .keys(doc! { "stream": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        Ok(())
    }

    fn events(&self) -> &Collection<Document> {
        &self.events
    }

    fn versions(&self) -> &Collection<Document> {
        &self.versions
    }
}

#[async_trait]
impl EventStore for MongoDbEventStore {
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        let filter = doc! {
            "aggregate_type": aggregate_type,
            "aggregate_id": aggregate_id,
        };

        let mut cursor = self
            .events()
            .find(filter)
            .sort(doc! { "version": 1 })
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        let mut events = Vec::new();
        while let Some(result) = cursor.next().await {
            let doc = result.map_err(|e| EventStoreError::Storage(e.to_string()))?;
            events.push(document_to_stored(&doc)?);
        }

        Ok(events)
    }

    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: Version,
        events: Vec<StoredEvent>,
    ) -> Result<(), EventStoreError> {
        // Valida a sequência de versões antes de tocar no banco.
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

        let last_version = events.last().map(|e| e.version).unwrap_or(expected_version);
        let stream_key = format!("{}:{}", aggregate_type, aggregate_id);

        // Atualização atômica: só prossegue se a versão atual confere
        // com `expected_version`. Para nova stream (expected = 0), usa
        // upsert para criar o documento de controle.
        let filter = doc! {
            "stream": &stream_key,
            "version": expected_version.value() as i64,
        };
        let update = doc! {
            "$set": {
                "stream": &stream_key,
                "version": last_version.value() as i64,
            }
        };

        let result = self
            .versions()
            .update_one(filter, update)
            .upsert(expected_version == Version::initial())
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        let matched = result.matched_count;
        let upserted = result.upserted_id.is_some();

        if matched == 0 && !upserted {
            // Conflito: a versão atual não é a esperada. Busca a real
            // para preencher o erro.
            let current = self
                .versions()
                .find_one(doc! { "stream": &stream_key })
                .await
                .map_err(|e| EventStoreError::Storage(e.to_string()))?;

            let actual = current
                .and_then(|d| d.get_i64("version").ok())
                .map(|v| Version::new(v.max(0) as u64))
                .unwrap_or(Version::initial());

            return Err(EventStoreError::Concurrency(ConcurrencyError {
                expected: expected_version,
                actual,
            }));
        }

        // Persiste todos os eventos.
        let mut docs = Vec::with_capacity(events.len());
        for event in &events {
            docs.push(stored_to_document(event)?);
        }

        self.events()
            .insert_many(docs)
            .await
            .map_err(|e| EventStoreError::Storage(e.to_string()))?;

        Ok(())
    }
}

/// Converte um [`StoredEvent`] em um [`Document`] do MongoDB.
fn stored_to_document(event: &StoredEvent) -> Result<Document, EventStoreError> {
    bson::to_document(event).map_err(|e| EventStoreError::Storage(e.to_string()))
}

/// Converte um [`Document`] do MongoDB em um [`StoredEvent`].
fn document_to_stored(doc: &Document) -> Result<StoredEvent, EventStoreError> {
    bson::from_document(doc.clone())
        .map_err(|e| EventStoreError::Storage(e.to_string()))
}