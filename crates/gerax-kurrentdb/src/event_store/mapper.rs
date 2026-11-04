//! Conversão entre os tipos do Gerax e os tipos nativos do KurrentDB.

use chrono::{DateTime, Utc};
use kurrentdb::{EventData, RecordedEvent, StreamState};
use serde::{Deserialize, Serialize};

use gerax_es::error::EventStoreError;
use gerax_es::event::{EventId, StoredEvent};
use gerax_es::metadata::EventMetadata;
use gerax_es::version::Version;

/// Metadata interna preservada no *custom metadata* do KurrentDB.
///
/// O evento nativo do KurrentDB não carrega identidade do Aggregate nem
/// o instante original do Domain Event. Esses dados são preservados em
/// JSON no custom metadata, junto com a [`EventMetadata`] extensível.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct KurrentEventMetadata {
    pub(crate) aggregate_type: String,
    pub(crate) aggregate_id: String,
    pub(crate) occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub(crate) metadata: EventMetadata,
}

/// Converte uma [`Version`] do Gerax para o estado de stream esperado
/// pelo KurrentDB.
///
/// | Gerax | KurrentDB |
/// |---|---|
/// | `0` | `NoStream` |
/// | `1` | `StreamRevision(0)` |
/// | `n` | `StreamRevision(n - 1)` |
pub(crate) fn stream_state(version: Version) -> StreamState {
    match version.value() {
        0 => StreamState::NoStream,
        value => StreamState::StreamRevision(value - 1),
    }
}

/// Converte um [`StoredEvent`] para o formato de evento do KurrentDB.
pub(crate) fn stored_to_event_data(event: &StoredEvent) -> Result<EventData, EventStoreError> {
    let envelope = KurrentEventMetadata {
        aggregate_type: event.aggregate_type.clone(),
        aggregate_id: event.aggregate_id.clone(),
        occurred_at: event.occurred_at,
        metadata: event.metadata.clone(),
    };

    let data = EventData::json(&event.event_type, &event.payload)
        .map_err(|error| EventStoreError::Serialization(error.to_string()))?
        .id(event.event_id.as_uuid());

    data.metadata_as_json(&envelope)
        .map_err(|error| EventStoreError::Serialization(error.to_string()))
}

/// Converte um evento lido do KurrentDB para [`StoredEvent`].
pub(crate) fn recorded_to_stored(
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

/// Campos de um evento lido, isolados do tipo do cliente para permitir
/// testes sem construir um [`RecordedEvent`].
pub(crate) struct RecordedEventFields<'a> {
    pub(crate) event_id: uuid::Uuid,
    pub(crate) revision: u64,
    pub(crate) event_type: &'a str,
    pub(crate) data: &'a [u8],
    pub(crate) custom_metadata: &'a [u8],
    pub(crate) created: DateTime<Utc>,
}

/// Converte os campos de um evento lido para [`StoredEvent`].
pub(crate) fn recorded_fields_to_stored(
    fields: RecordedEventFields<'_>,
    aggregate_type: &str,
    aggregate_id: &str,
) -> Result<StoredEvent, EventStoreError> {
    let payload: serde_json::Value = serde_json::from_slice(fields.data)
        .map_err(|error| EventStoreError::Serialization(error.to_string()))?;

    let envelope = decode_custom_metadata(fields.custom_metadata)?;

    let (stored_aggregate_type, stored_aggregate_id, metadata, occurred_at) = match envelope {
        Some(envelope) => (
            if envelope.aggregate_type.is_empty() {
                aggregate_type.to_string()
            } else {
                envelope.aggregate_type
            },
            if envelope.aggregate_id.is_empty() {
                aggregate_id.to_string()
            } else {
                envelope.aggregate_id
            },
            envelope.metadata,
            envelope.occurred_at,
        ),
        None => (
            aggregate_type.to_string(),
            aggregate_id.to_string(),
            EventMetadata::new(),
            fields.created,
        ),
    };

    Ok(StoredEvent {
        event_id: EventId::from_uuid(fields.event_id),
        aggregate_id: stored_aggregate_id,
        aggregate_type: stored_aggregate_type,
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
pub(crate) fn decode_custom_metadata(
    custom_metadata: &[u8],
) -> Result<Option<KurrentEventMetadata>, EventStoreError> {
    if custom_metadata.is_empty() {
        return Ok(None);
    }

    serde_json::from_slice(custom_metadata)
        .map(Some)
        .map_err(|error| EventStoreError::Serialization(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use kurrentdb::CurrentRevision;

    fn stored(version: u64) -> StoredEvent {
        let mut metadata = EventMetadata::new();
        metadata.insert("correlation_id", "fluxo-1");

        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "aluno-1".into(),
            aggregate_type: "Aluno".into(),
            event_type: "AlunoMatriculado".into(),
            version: Version::new(version),
            payload: serde_json::json!({ "aluno_id": "aluno-1" }),
            metadata,
            occurred_at: Utc::now(),
        }
    }

    fn fields<'a>(data: &'a [u8], custom_metadata: &'a [u8]) -> RecordedEventFields<'a> {
        RecordedEventFields {
            event_id: uuid::Uuid::new_v4(),
            revision: 4,
            event_type: "AlunoMatriculado",
            data,
            custom_metadata,
            created: Utc::now(),
        }
    }

    #[test]
    fn versao_zero_vira_no_stream() {
        assert!(matches!(
            stream_state(Version::initial()),
            StreamState::NoStream
        ));
    }

    #[test]
    fn versao_gerax_corresponde_a_revisao_anterior() {
        assert!(matches!(
            stream_state(Version::new(1)),
            StreamState::StreamRevision(0)
        ));
        assert!(matches!(
            stream_state(Version::new(7)),
            StreamState::StreamRevision(6)
        ));
    }

    #[test]
    fn stored_event_vira_event_data() {
        assert!(stored_to_event_data(&stored(3)).is_ok());
    }

    #[test]
    fn evento_lido_preserva_versao_metadata_e_payload() {
        let original = stored(5);
        let data = serde_json::to_vec(&original.payload).unwrap();
        let envelope = KurrentEventMetadata {
            aggregate_type: original.aggregate_type.clone(),
            aggregate_id: original.aggregate_id.clone(),
            occurred_at: original.occurred_at,
            metadata: original.metadata.clone(),
        };
        let custom_metadata = serde_json::to_vec(&envelope).unwrap();

        let event =
            recorded_fields_to_stored(fields(&data, &custom_metadata), "Aluno", "aluno-1").unwrap();

        assert_eq!(event.event_type, "AlunoMatriculado");
        assert_eq!(event.version, Version::new(5));
        assert_eq!(event.aggregate_type, "Aluno");
        assert_eq!(event.aggregate_id, "aluno-1");
        assert_eq!(event.payload, original.payload);
        assert_eq!(
            event.metadata.get("correlation_id"),
            Some(&serde_json::Value::from("fluxo-1"))
        );
    }

    #[test]
    fn evento_externo_sem_metadata_usa_fallback_do_stream() {
        let created = Utc::now();
        let fields = RecordedEventFields {
            event_id: uuid::Uuid::new_v4(),
            revision: 0,
            event_type: "AlunoMatriculado",
            data: br#"{"aluno_id":"aluno-1"}"#,
            custom_metadata: b"",
            created,
        };

        let event = recorded_fields_to_stored(fields, "Aluno", "aluno-1").unwrap();
        assert_eq!(event.version, Version::new(1));
        assert_eq!(event.aggregate_type, "Aluno");
        assert_eq!(event.aggregate_id, "aluno-1");
        assert_eq!(event.occurred_at, created);
        assert!(event.metadata.is_empty());
    }

    #[test]
    fn custom_metadata_invalido_nao_e_ignorado() {
        let result = decode_custom_metadata(b"{nao-e-json");
        assert!(matches!(result, Err(EventStoreError::Serialization(_))));
    }

    #[test]
    fn custom_metadata_ausente_retorna_none() {
        assert!(decode_custom_metadata(b"").unwrap().is_none());
    }

    #[test]
    fn revisao_do_kurrentdb_vira_versao_gerax_somando_um() {
        let fields = RecordedEventFields {
            event_id: uuid::Uuid::new_v4(),
            revision: 41,
            event_type: "AlunoMatriculado",
            data: br#"{"aluno_id":"aluno-1"}"#,
            custom_metadata: b"",
            created: Utc::now(),
        };

        let event = recorded_fields_to_stored(fields, "Aluno", "aluno-1").unwrap();
        assert_eq!(event.version, Version::new(42));
    }

    #[test]
    fn current_revision_e_mapeada_pela_store() {
        // Documenta a semântica usada no mapeamento de conflito.
        let current = CurrentRevision::Current(4);
        let version = match current {
            CurrentRevision::Current(revision) => Version::new(revision + 1),
            CurrentRevision::NoStream => Version::initial(),
        };
        assert_eq!(version, Version::new(5));
    }
}
