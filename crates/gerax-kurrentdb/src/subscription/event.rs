//! Evento recebido por uma Persistent Subscription.
//!
//! [`SubscriptionEvent`] é a representação **independente do cliente**
//! entregue à camada superior. Ele preserva os bytes originais do
//! payload e do custom metadata (para que a `gerax-eda` construa o seu
//! próprio envelope de integração) e expõe uma conversão explícita para
//! [`StoredEvent`].

use chrono::{DateTime, Utc};
use kurrentdb::RecordedEvent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use gerax_es::event::{EventId, StoredEvent};

use crate::event_store::mapper::{RecordedEventFields, recorded_fields_to_stored};
use crate::event_store::stream::parse_stream_name;

use super::error::SubscriptionError;

/// Evento recebido de uma Persistent Subscription.
///
/// ```text
/// SubscriptionEvent
///  ├── id               EventId original (preservado no ACK/NACK)
///  ├── event_type       tipo lógico do evento
///  ├── stream_id        nome da stream de origem
///  ├── stream_revision  revisão original (0-based)
///  ├── payload          bytes originais do payload
///  ├── metadata         bytes originais do custom metadata
///  ├── aggregate_*      identidade do Aggregate de origem
///  ├── occurred_at      instante original do evento
///  └── retry_count      tentativas anteriores de entrega
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionEvent {
    /// Identificador original do evento.
    pub id: EventId,
    /// Tipo lógico do evento.
    pub event_type: String,
    /// Nome da stream de origem.
    pub stream_id: String,
    /// Revisão original na stream (0-based, como no KurrentDB).
    pub stream_revision: u64,
    /// Payload original, em bytes.
    pub payload: Vec<u8>,
    /// Custom metadata original, em bytes.
    pub metadata: Vec<u8>,
    /// Tipo do Aggregate de origem, quando identificável.
    pub aggregate_type: String,
    /// Identificador do Aggregate de origem, quando identificável.
    pub aggregate_id: String,
    /// Instante original em que o evento ocorreu.
    pub occurred_at: DateTime<Utc>,
    /// Quantidade de entregas anteriores (retry count do servidor).
    pub retry_count: u32,
    /// Posição global do evento no `$all`, quando disponível.
    pub commit_position: Option<u64>,
}

impl SubscriptionEvent {
    /// Converte um evento nativo do KurrentDB no tipo independente do
    /// adapter.
    ///
    /// A identidade do Aggregate é obtida do custom metadata do adapter
    /// quando presente; caso contrário, é interpretada a partir do nome
    /// da stream (`{aggregate_type}-{aggregate_id}`).
    pub fn from_recorded(
        record: &RecordedEvent,
        retry_count: u32,
        commit_position: Option<u64>,
    ) -> Result<Self, SubscriptionError> {
        let (fallback_type, fallback_id) =
            parse_stream_name(record.stream_id()).unwrap_or(("", ""));

        let stored = recorded_fields_to_stored(
            RecordedEventFields {
                event_id: record.id,
                revision: record.revision,
                event_type: &record.event_type,
                data: &record.data,
                custom_metadata: &record.custom_metadata,
                created: record.created,
            },
            fallback_type,
            fallback_id,
        )
        .map_err(|error| SubscriptionError::Serialization(error.to_string()))?;

        Ok(Self {
            id: stored.event_id,
            event_type: stored.event_type,
            stream_id: record.stream_id().to_string(),
            stream_revision: record.revision,
            payload: record.data.to_vec(),
            metadata: record.custom_metadata.to_vec(),
            aggregate_type: stored.aggregate_type,
            aggregate_id: stored.aggregate_id,
            occurred_at: stored.occurred_at,
            retry_count,
            commit_position,
        })
    }

    /// Desserializa o payload original como JSON.
    pub fn payload_json(&self) -> Result<Value, SubscriptionError> {
        serde_json::from_slice(&self.payload)
            .map_err(|error| SubscriptionError::Serialization(error.to_string()))
    }

    /// Reconstrói o [`StoredEvent`] equivalente.
    ///
    /// A versão Gerax é `stream_revision + 1` e o custom metadata é
    /// interpretado exatamente como no Event Store.
    pub fn to_stored_event(&self) -> Result<StoredEvent, SubscriptionError> {
        recorded_fields_to_stored(
            RecordedEventFields {
                event_id: self.id.as_uuid(),
                revision: self.stream_revision,
                event_type: &self.event_type,
                data: &self.payload,
                custom_metadata: &self.metadata,
                created: self.occurred_at,
            },
            &self.aggregate_type,
            &self.aggregate_id,
        )
        .map_err(|error| SubscriptionError::Serialization(error.to_string()))
    }

    /// Identificador original do evento.
    pub fn event_id(&self) -> &EventId {
        &self.id
    }

    /// `true` quando o evento já foi reentregue ao menos uma vez.
    pub fn is_redelivery(&self) -> bool {
        self.retry_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;

    use crate::event_store::mapper::KurrentEventMetadata;

    fn subscription_event(
        stream: &str,
        event_type: &str,
        revision: u64,
        payload: &[u8],
        metadata: &[u8],
    ) -> SubscriptionEvent {
        let (aggregate_type, aggregate_id) = parse_stream_name(stream).unwrap_or(("", ""));

        SubscriptionEvent {
            id: EventId::new(),
            event_type: event_type.to_string(),
            stream_id: stream.to_string(),
            stream_revision: revision,
            payload: payload.to_vec(),
            metadata: metadata.to_vec(),
            aggregate_type: aggregate_type.to_string(),
            aggregate_id: aggregate_id.to_string(),
            occurred_at: Utc::now(),
            retry_count: 0,
            commit_position: Some(42),
        }
    }

    #[test]
    fn identidade_e_revisao_sao_preservadas() {
        let event = subscription_event("Aluno-aluno-1", "AlunoMatriculado", 0, br#"{"a":1}"#, b"");

        assert_eq!(event.event_type, "AlunoMatriculado");
        assert_eq!(event.stream_id, "Aluno-aluno-1");
        assert_eq!(event.stream_revision, 0);
        assert_eq!(event.aggregate_type, "Aluno");
        assert_eq!(event.aggregate_id, "aluno-1");
        assert_eq!(event.payload, br#"{"a":1}"#);
        assert_eq!(event.commit_position, Some(42));
        assert_eq!(event.payload_json().unwrap(), serde_json::json!({"a": 1}));
    }

    #[test]
    fn custom_metadata_do_adapter_tem_precedencia() {
        let envelope = KurrentEventMetadata {
            aggregate_type: "Turma".into(),
            aggregate_id: "turma-9".into(),
            occurred_at: Utc::now(),
            metadata: gerax_es::EventMetadata::new(),
        };
        let metadata = serde_json::to_vec(&envelope).unwrap();
        let mut event = subscription_event("Outro-stream", "TurmaCriada", 6, br#"{}"#, &metadata);
        event.retry_count = 2;

        let stored = event.to_stored_event().unwrap();
        assert_eq!(stored.aggregate_type, "Turma");
        assert_eq!(stored.aggregate_id, "turma-9");
        assert_eq!(stored.occurred_at, envelope.occurred_at);
        assert_eq!(event.retry_count, 2);
        assert!(event.is_redelivery());
    }

    #[test]
    fn stored_event_e_reconstruido_com_versao_gerax() {
        let event = subscription_event("Aluno-aluno-1", "AlunoMatriculado", 4, br#"{"a":1}"#, b"");
        let stored = event.to_stored_event().unwrap();

        assert_eq!(stored.event_id, event.id);
        assert_eq!(stored.version, gerax_es::Version::new(5));
        assert_eq!(stored.aggregate_type, "Aluno");
        assert_eq!(stored.aggregate_id, "aluno-1");
        assert_eq!(stored.event_type, "AlunoMatriculado");
        assert_eq!(stored.payload, serde_json::json!({"a": 1}));
    }

    #[test]
    fn payload_invalido_vira_erro_de_serializacao() {
        let event = subscription_event("Aluno-a", "AlunoMatriculado", 0, b"nao-json", b"");
        assert!(matches!(
            event.to_stored_event(),
            Err(SubscriptionError::Serialization(_))
        ));
    }

    #[test]
    fn stream_sem_separador_nao_falha() {
        let event = subscription_event("$all", "System", 0, br#"{}"#, b"");
        assert_eq!(event.aggregate_type, "");
        assert_eq!(event.aggregate_id, "");
    }
}
