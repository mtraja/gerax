//! Adapter de [`EventStore`] para KurrentDB.
//!
//! Usa o client oficial `kurrentdb` (gRPC) para persistência em
//! [KurrentDB](https://kurrent.io/) — o antigo EventStoreDB.
//!
//! ## Streams
//!
//! Cada Aggregate é persistido em uma stream própria, nomeada como:
//!
//! ```text
//! {aggregate_type}-{aggregate_id}
//! ```
//!
//! Por exemplo, `Student` com id `student-1` vira a stream
//! `Student-student-1`.
//!
//! ## Versões
//!
//! O [`Version`] do Gerax começa em `0` e é incrementado a cada evento
//! (`1`, `2`, ...). O KurrentDB usa revisões baseadas em `0`, onde a
//! primeira revisão é `0` e a ausência de stream é representada por
//! `StreamState::NoStream`. O adapter converte entre os dois modelos:
//!
//! ```text
//! Gerax Version 0  ↔  StreamState::NoStream
//! Gerax Version 1  ↔  StreamState::StreamRevision(0)
//! Gerax Version 2  ↔  StreamState::StreamRevision(1)
//! ```
//!
//! ## Concorrência
//!
//! O `append` envia o `StreamState` esperado ao servidor. Se a stream já
//! tiver avançado, o KurrentDB responde com `WrongExpectedVersion`,
//! convertido para [`EventStoreError::Concurrency`]. O adapter **não**
//! faz retry automático: a aplicação recarrega o Aggregate e reexecuta o
//! Command.
//!
//! ## Metadata
//!
//! O tipo nativo do KurrentDB não possui campos para o `aggregate_type`,
//! `aggregate_id` ou para o instante original do evento. Esses dados,
//! junto com a [`EventMetadata`], são preservados no *custom metadata* do
//! evento (JSON). O payload do Domain Event fica no *data* do evento.
//!
//! ## Uso
//!
//! ```ignore
//! use gerax_es::{EventStore, EventSourcedRepository, JsonEventSerializer};
//! use gerax_kurrentdb::{KurrentDbClient, KurrentDbEventStore};
//!
//! let client = KurrentDbClient::connect(
//!     "kurrentdb://admin:changeit@localhost:2113?tls=false",
//! ).await?;
//! let store = KurrentDbEventStore::new(client);
//! let repo = EventSourcedRepository::new(store, JsonEventSerializer::new());
//! ```

use kurrentdb::{AppendToStreamOptions, Client, CurrentRevision, ReadStreamOptions};

use gerax_es::error::{ConcurrencyError, EventStoreError};
use gerax_es::event::StoredEvent;
use gerax_es::event_store::EventStore;
use gerax_es::version::Version;
use tracing::{debug, warn};

use crate::client::KurrentDbClient;
use crate::config::KurrentDbConfig;

use super::mapper::{recorded_to_stored, stored_to_event_data, stream_state};
use super::stream::stream_name;

/// Adapter de [`EventStore`] para KurrentDB.
///
/// Cada stream é identificada pelo par `(aggregate_type, aggregate_id)`
/// e os eventos são mantidos em ordem de revisão.
#[doc(alias = "KurrentDB")]
#[doc(alias = "EventStoreDB")]
pub struct KurrentDbEventStore {
    client: KurrentDbClient,
}

/// Alias de compatibilidade para o nome anterior, quando o banco ainda
/// se chamava EventStoreDB.
#[deprecated(note = "use `KurrentDbEventStore`")]
pub type EventStoreDbEventStore = KurrentDbEventStore;

impl KurrentDbEventStore {
    /// Cria o Event Store a partir de um cliente compartilhado.
    pub fn new(client: KurrentDbClient) -> Self {
        Self { client }
    }

    /// Cria o Event Store a partir de um [`Client`] já configurado.
    pub fn from_raw_client(client: Client) -> Self {
        Self::new(KurrentDbClient::new(client))
    }

    /// Conecta ao KurrentDB a partir de uma connection string.
    ///
    /// Exemplos de connection string:
    ///
    /// ```text
    /// kurrentdb://localhost:2113?tls=false
    /// kurrentdb://admin:changeit@localhost:2113?tls=false
    /// kurrentdb://admin:changeit@node-1:2113,node-2:2113
    /// ```
    ///
    /// O client também aceita o esquema legado `esdb://`.
    pub async fn connect(connection_string: &str) -> Result<Self, EventStoreError> {
        let client = KurrentDbClient::connect(connection_string).await?;
        Ok(Self::new(client))
    }

    /// Conecta a partir da configuração do adapter.
    pub async fn from_config(config: &KurrentDbConfig) -> Result<Self, EventStoreError> {
        let client = KurrentDbClient::from_config(config).await?;
        Ok(Self::new(client))
    }

    /// Cliente compartilhado subjacente.
    pub fn kurrent_client(&self) -> &KurrentDbClient {
        &self.client
    }

    /// Cliente oficial subjacente.
    pub fn client(&self) -> &Client {
        self.client.client()
    }

    /// Nome da stream de um Aggregate.
    fn stream_name(aggregate_type: &str, aggregate_id: &str) -> String {
        stream_name(aggregate_type, aggregate_id)
    }
}

#[async_trait::async_trait]
impl EventStore for KurrentDbEventStore {
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<StoredEvent>, EventStoreError> {
        let stream = Self::stream_name(aggregate_type, aggregate_id);
        let options = ReadStreamOptions::default().forwards();

        let mut read = match self
            .client
            .client()
            .read_stream(stream.as_str(), &options)
            .await
        {
            Ok(read) => read,
            Err(kurrentdb::Error::ResourceNotFound) => return Ok(Vec::new()),
            Err(error) => return Err(map_read_error(error)),
        };

        let mut events = Vec::new();

        loop {
            match read.next().await {
                Ok(Some(resolved)) => {
                    let event = resolved.get_original_event();
                    events.push(recorded_to_stored(event, aggregate_type, aggregate_id)?);
                }
                Ok(None) => break,
                Err(kurrentdb::Error::ResourceNotFound) => return Ok(Vec::new()),
                Err(error) => return Err(map_read_error(error)),
            }
        }

        debug!(
            aggregate_type,
            aggregate_id,
            stream = %stream,
            event_count = events.len(),
            "eventos carregados do KurrentDB"
        );

        Ok(events)
    }

    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: Version,
        events: Vec<StoredEvent>,
    ) -> Result<(), EventStoreError> {
        validate_sequence(expected_version, &events)?;

        if events.is_empty() {
            return Ok(());
        }

        let stream = Self::stream_name(aggregate_type, aggregate_id);
        let options = AppendToStreamOptions::default().stream_state(stream_state(expected_version));

        let data = events
            .iter()
            .map(stored_to_event_data)
            .collect::<Result<Vec<_>, _>>()?;

        self.client
            .client()
            .append_to_stream(stream.clone(), &options, data)
            .await
            .map_err(|error| map_append_error(error, expected_version))?;

        debug!(
            aggregate_type,
            aggregate_id,
            stream = %stream,
            expected_version = expected_version.value(),
            appended = events.len(),
            "eventos persistidos no KurrentDB"
        );

        Ok(())
    }
}

/// Valida se os eventos formam uma sequência contínua a partir de
/// `expected_version`.
fn validate_sequence(
    expected_version: Version,
    events: &[StoredEvent],
) -> Result<(), EventStoreError> {
    let mut next = expected_version.next();

    for event in events {
        if event.version != next {
            return Err(EventStoreError::InvalidSequence(format!(
                "expected version {}, got {}",
                next, event.version
            )));
        }
        next = next.next();
    }

    Ok(())
}

/// Converte erros de leitura em erros da porta [`EventStore`].
fn map_read_error(error: kurrentdb::Error) -> EventStoreError {
    match error {
        kurrentdb::Error::ResourceNotFound => EventStoreError::ResourceNotFound("stream not found".into()),
        kurrentdb::Error::ResourceDeleted => EventStoreError::ResourceDeleted("stream was deleted".into()),
        kurrentdb::Error::AccessDenied => EventStoreError::Unauthorized("access denied".into()),
        kurrentdb::Error::ConnectionClosed
        | kurrentdb::Error::GrpcConnectionError(_)
        | kurrentdb::Error::NotLeaderException(_)
        | kurrentdb::Error::DeadlineExceeded => EventStoreError::Unavailable(error.to_string()),
        other => EventStoreError::Storage(other.to_string()),
    }
}

/// Converte erros do KurrentDB em erros da porta [`EventStore`].
fn map_append_error(error: kurrentdb::Error, expected: Version) -> EventStoreError {
    match error {
        kurrentdb::Error::WrongExpectedVersion { current, .. } => {
            let actual = match current {
                CurrentRevision::Current(revision) => Version::new(revision.saturating_add(1)),
                CurrentRevision::NoStream => Version::initial(),
            };

            warn!(
                expected_version = expected.value(),
                actual_version = actual.value(),
                "conflito de concorrência no append"
            );

            EventStoreError::Concurrency(ConcurrencyError { expected, actual })
        }
        kurrentdb::Error::ConnectionClosed
        | kurrentdb::Error::GrpcConnectionError(_)
        | kurrentdb::Error::NotLeaderException(_)
        | kurrentdb::Error::DeadlineExceeded => EventStoreError::Unavailable(error.to_string()),
        kurrentdb::Error::AccessDenied => EventStoreError::Unauthorized("access denied".into()),
        other => EventStoreError::Storage(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;

    use gerax_es::event::{EventId, StoredEvent};
    use gerax_es::metadata::EventMetadata;

    fn stored_event(version: u64) -> StoredEvent {
        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "aluno-1".into(),
            aggregate_type: "Aluno".into(),
            event_type: "AlunoMatriculado".into(),
            version: Version::new(version),
            payload: serde_json::json!({}),
            metadata: EventMetadata::new(),
            occurred_at: Utc::now(),
        }
    }

    #[test]
    fn sequencia_continua_e_aceita() {
        let events = vec![stored_event(3), stored_event(4), stored_event(5)];
        assert!(validate_sequence(Version::new(2), &events).is_ok());
    }

    #[test]
    fn sequencia_com_lacuna_e_rejeitada() {
        let events = vec![stored_event(3), stored_event(5)];
        let error = validate_sequence(Version::new(2), &events).unwrap_err();
        assert!(matches!(error, EventStoreError::InvalidSequence(_)));
    }

    #[test]
    fn sequencia_que_nao_comeca_na_versao_esperada_e_rejeitada() {
        let events = vec![stored_event(1)];
        let error = validate_sequence(Version::new(2), &events).unwrap_err();
        assert!(matches!(error, EventStoreError::InvalidSequence(_)));
    }

    #[test]
    fn lista_vazia_e_uma_sequencia_valida() {
        assert!(validate_sequence(Version::new(10), &[]).is_ok());
    }
}
