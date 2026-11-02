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
//! convertido para [`EventStoreError::Concurrency`].
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
//! use gerax_es::adapters::kurrentdb::KurrentDbEventStore;
//!
//! let store = KurrentDbEventStore::connect(
//!     "kurrentdb://admin:changeit@localhost:2113?tls=false",
//! ).await?;
//! let repo = EventSourcedRepository::new(store, JsonEventSerializer::new());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use ::kurrentdb::{
    AppendToStreamOptions, Client, ClientSettings, CurrentRevision, EventData, ReadStreamOptions,
    RecordedEvent, StreamState,
};

use crate::error::{ConcurrencyError, EventStoreError};
use crate::event::{EventId, StoredEvent};
use crate::event_store::EventStore;
use crate::metadata::EventMetadata;
use crate::version::Version;

/// Adapter de [`EventStore`] para KurrentDB.
///
/// Cada stream é identificada pelo par `(aggregate_type, aggregate_id)`
/// e os eventos são mantidos em ordem de revisão.
#[doc(alias = "KurrentDB")]
#[doc(alias = "EventStoreDB")]
pub struct KurrentDbEventStore {
    client: Client,
}

/// Alias de compatibilidade para o nome anterior, quando o banco ainda
/// se chamava EventStoreDB.
#[deprecated(note = "use `KurrentDbEventStore`")]
pub type EventStoreDbEventStore = KurrentDbEventStore;

impl KurrentDbEventStore {
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
        let settings: ClientSettings =
            connection_string
                .parse()
                .map_err(|e: ::kurrentdb::ClientSettingsParseError| {
                    EventStoreError::Storage(e.to_string())
                })?;

        let client = Client::new(settings).map_err(|e| EventStoreError::Storage(e.to_string()))?;

        Ok(Self { client })
    }

    /// Cria um store a partir de um [`Client`] já configurado.
    ///
    /// Útil quando a aplicação precisa customizar as opções de conexão,
    /// autenticação ou TLS.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Retorna referência ao client subjacente.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Nome da stream de um Aggregate.
    fn stream_name(aggregate_type: &str, aggregate_id: &str) -> String {
        format!("{aggregate_type}-{aggregate_id}")
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

        let mut read = match self.client.read_stream(stream, &options).await {
            Ok(read) => read,
            Err(::kurrentdb::Error::ResourceNotFound) => return Ok(Vec::new()),
            Err(error) => return Err(EventStoreError::Storage(error.to_string())),
        };

        let mut events = Vec::new();

        loop {
            match read.next().await {
                Ok(Some(resolved)) => {
                    let event = resolved.get_original_event();
                    events.push(recorded_to_stored(event, aggregate_type, aggregate_id)?);
                }
                Ok(None) => break,
                Err(::kurrentdb::Error::ResourceNotFound) => return Ok(Vec::new()),
                Err(error) => return Err(EventStoreError::Storage(error.to_string())),
            }
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
            .append_to_stream(stream, &options, data)
            .await
            .map_err(|error| map_append_error(error, expected_version))?;

        Ok(())
    }
}

/// Metadata interna preservada no *custom metadata* do KurrentDB.
#[derive(Debug, Serialize, Deserialize)]
struct KurrentEventMetadata {
    aggregate_type: String,
    aggregate_id: String,
    occurred_at: DateTime<Utc>,
    #[serde(default)]
    metadata: EventMetadata,
}

/// Converte uma [`Version`] do Gerax para o estado de stream esperado
/// pelo KurrentDB.
fn stream_state(version: Version) -> StreamState {
    match version.value() {
        0 => StreamState::NoStream,
        value => StreamState::StreamRevision(value - 1),
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

/// Converte um [`StoredEvent`] para o formato de evento do KurrentDB.
fn stored_to_event_data(event: &StoredEvent) -> Result<EventData, EventStoreError> {
    let envelope = KurrentEventMetadata {
        aggregate_type: event.aggregate_type.clone(),
        aggregate_id: event.aggregate_id.clone(),
        occurred_at: event.occurred_at,
        metadata: event.metadata.clone(),
    };

    let data = EventData::json(&event.event_type, &event.payload)
        .map_err(|e| EventStoreError::Serialization(e.to_string()))?
        .id(event.event_id.as_uuid())
        .metadata_as_json(&envelope)
        .map_err(|e| EventStoreError::Serialization(e.to_string()))?;

    Ok(data)
}

/// Converte um evento lido do KurrentDB para [`StoredEvent`].
fn recorded_to_stored(
    record: &RecordedEvent,
    aggregate_type: &str,
    aggregate_id: &str,
) -> Result<StoredEvent, EventStoreError> {
    recorded_fields_to_stored(
        RecordedEventFields {
            event_id: record.id,
            revision: record.revision,
            event_type: &record.event_type,
            data: &record.data,
            custom_metadata: &record.custom_metadata,
            created: record.created,
        },
        aggregate_type,
        aggregate_id,
    )
}

/// Campos de um evento lido, isolados do tipo do client para permitir
/// testes sem construir um [`RecordedEvent`].
struct RecordedEventFields<'a> {
    event_id: uuid::Uuid,
    revision: u64,
    event_type: &'a str,
    data: &'a [u8],
    custom_metadata: &'a [u8],
    created: DateTime<Utc>,
}

/// Converte os campos de um evento lido para [`StoredEvent`].
fn recorded_fields_to_stored(
    fields: RecordedEventFields<'_>,
    aggregate_type: &str,
    aggregate_id: &str,
) -> Result<StoredEvent, EventStoreError> {
    let payload: serde_json::Value = serde_json::from_slice(fields.data)
        .map_err(|e| EventStoreError::Serialization(e.to_string()))?;

    let envelope = decode_custom_metadata(fields.custom_metadata)?;

    let (metadata, occurred_at) = match envelope {
        Some(envelope) => (envelope.metadata, envelope.occurred_at),
        None => (EventMetadata::new(), fields.created),
    };

    Ok(StoredEvent {
        event_id: EventId::from_uuid(fields.event_id),
        aggregate_id: aggregate_id.to_string(),
        aggregate_type: aggregate_type.to_string(),
        event_type: fields.event_type.to_string(),
        version: Version::new(fields.revision.saturating_add(1)),
        payload,
        metadata,
        occurred_at,
    })
}

/// Decodifica o *custom metadata* gravado pelo adapter.
///
/// Eventos externos (sem custom metadata) são aceitos e mapeados para
/// metadata vazia. Um custom metadata presente porém inválido é tratado
/// como erro de serialização — nunca ignorado silenciosamente.
fn decode_custom_metadata(
    custom_metadata: &[u8],
) -> Result<Option<KurrentEventMetadata>, EventStoreError> {
    if custom_metadata.is_empty() {
        return Ok(None);
    }

    serde_json::from_slice(custom_metadata)
        .map(Some)
        .map_err(|e| EventStoreError::Serialization(e.to_string()))
}

/// Converte erros do KurrentDB em erros da porta [`EventStore`].
fn map_append_error(error: ::kurrentdb::Error, expected: Version) -> EventStoreError {
    match error {
        ::kurrentdb::Error::WrongExpectedVersion { current, .. } => {
            let actual = match current {
                CurrentRevision::Current(revision) => Version::new(revision.saturating_add(1)),
                CurrentRevision::NoStream => Version::initial(),
            };

            EventStoreError::Concurrency(ConcurrencyError { expected, actual })
        }
        other => EventStoreError::Storage(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored_event(version: u64, event_type: &str) -> StoredEvent {
        let mut metadata = EventMetadata::new();
        metadata.insert("correlation_id", "corr-1");

        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "student-1".into(),
            aggregate_type: "Student".into(),
            event_type: event_type.into(),
            version: Version::new(version),
            payload: serde_json::json!({ "name": "Ana" }),
            metadata,
            occurred_at: Utc::now(),
        }
    }

    /// Evento lido simulando a resposta do servidor.
    struct TestRecord {
        event_id: uuid::Uuid,
        revision: u64,
        event_type: String,
        data: Vec<u8>,
        custom_metadata: Vec<u8>,
        created: DateTime<Utc>,
    }

    impl TestRecord {
        fn fields(&self) -> RecordedEventFields<'_> {
            RecordedEventFields {
                event_id: self.event_id,
                revision: self.revision,
                event_type: &self.event_type,
                data: &self.data,
                custom_metadata: &self.custom_metadata,
                created: self.created,
            }
        }
    }

    fn recorded_event(version: u64, custom_metadata: Vec<u8>) -> TestRecord {
        TestRecord {
            event_id: uuid::Uuid::new_v4(),
            revision: version - 1,
            event_type: "StudentCreated".into(),
            data: serde_json::to_vec(&serde_json::json!({ "name": "Ana" })).unwrap(),
            custom_metadata,
            created: Utc::now(),
        }
    }

    #[test]
    fn stream_name_combina_tipo_e_identificador() {
        assert_eq!(
            KurrentDbEventStore::stream_name("Student", "student-1"),
            "Student-student-1"
        );
    }

    #[test]
    fn version_inicial_vira_no_stream() {
        assert_eq!(stream_state(Version::initial()), StreamState::NoStream);
    }

    #[test]
    fn versoes_seguintes_vira_revisoes_zero_based() {
        assert_eq!(
            stream_state(Version::new(1)),
            StreamState::StreamRevision(0)
        );
        assert_eq!(
            stream_state(Version::new(7)),
            StreamState::StreamRevision(6)
        );
    }

    #[test]
    fn sequencia_continua_e_aceita() {
        let events = vec![
            stored_event(1, "StudentCreated"),
            stored_event(2, "StudentRenamed"),
        ];
        assert!(validate_sequence(Version::initial(), &events).is_ok());
    }

    #[test]
    fn sequencia_fora_de_ordem_e_rejeitada() {
        let events = vec![stored_event(2, "StudentRenamed")];
        let error = validate_sequence(Version::initial(), &events).unwrap_err();
        assert!(matches!(error, EventStoreError::InvalidSequence(_)));
    }

    #[test]
    fn evento_persistido_roundtrip_preserva_campos() {
        let event = stored_event(1, "StudentCreated");
        let envelope = KurrentEventMetadata {
            aggregate_type: event.aggregate_type.clone(),
            aggregate_id: event.aggregate_id.clone(),
            occurred_at: event.occurred_at,
            metadata: event.metadata.clone(),
        };
        let custom_metadata = serde_json::to_vec(&envelope).unwrap();

        let record = TestRecord {
            event_id: event.event_id.as_uuid(),
            event_type: event.event_type.clone(),
            data: serde_json::to_vec(&event.payload).unwrap(),
            custom_metadata,
            ..recorded_event(1, Vec::new())
        };

        let restored = recorded_fields_to_stored(record.fields(), "Student", "student-1").unwrap();
        assert_eq!(restored, event);
    }

    #[test]
    fn custom_metadata_ausente_usa_timestamp_do_servidor() {
        let record = recorded_event(3, Vec::new());
        let restored = recorded_fields_to_stored(record.fields(), "Student", "student-1").unwrap();

        assert_eq!(restored.version, Version::new(3));
        assert_eq!(restored.aggregate_type, "Student");
        assert_eq!(restored.aggregate_id, "student-1");
        assert_eq!(restored.occurred_at, record.created);
        assert!(restored.metadata.is_empty());
    }

    #[test]
    fn custom_metadata_invalido_gera_erro_de_serializacao() {
        let record = recorded_event(1, b"not-json".to_vec());
        let error = recorded_fields_to_stored(record.fields(), "Student", "student-1").unwrap_err();
        assert!(matches!(error, EventStoreError::Serialization(_)));
    }

    #[test]
    fn payload_invalido_gera_erro_de_serializacao() {
        let record = TestRecord {
            data: b"not-json".to_vec(),
            ..recorded_event(1, Vec::new())
        };
        let error = recorded_fields_to_stored(record.fields(), "Student", "student-1").unwrap_err();
        assert!(matches!(error, EventStoreError::Serialization(_)));
    }

    #[test]
    fn erro_de_versao_concorrente_vira_concurrency_error() {
        let error = map_append_error(
            ::kurrentdb::Error::WrongExpectedVersion {
                expected: StreamState::NoStream,
                current: CurrentRevision::Current(0),
            },
            Version::initial(),
        );

        match error {
            EventStoreError::Concurrency(ConcurrencyError { expected, actual }) => {
                assert_eq!(expected, Version::initial());
                assert_eq!(actual, Version::new(1));
            }
            other => panic!("esperado Concurrency, obtido {other:?}"),
        }
    }
}
