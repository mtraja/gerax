//! Envelope de transporte das mensagens (`§8`).
//!
//! Nenhum payload de evento é publicado "cru": o [`NatsEnvelope`]
//! empacota identidade, tipo, rastreabilidade (correlation/causation),
//! versionamento e o payload JSON. O corpo da mensagem é este envelope
//! serializado em JSON estável (`application/json`).
//!
//! O Gerax já possui o envelope tipado [`EventEnvelope`](gerax_eda::EventEnvelope)
//! (domínio → bus); o [`NatsEnvelope`] é a forma *em trânsito* — o
//! payload vira [`serde_json::Value`] para atravessar o wire, e o
//! consumidor o converte de volta ao tipo concreto.
//!
//! # Mapeamento para headers NATS (contrato do adapter)
//!
//! ```text
//! header NATS                    NatsEnvelope
//! ─────────────────────────────  ────────────────────────────────
//! Content-Type                   application/json (fixo)
//! Nats-Msg-Id                    event_id (UUID; dedup do JetStream)
//! x-causation-id                 causation_id
//! x-correlation-id               correlation_id
//! x-<key>                        headers[<key>]
//! ```
//!
//! `Nats-Msg-Id` habilita a **deduplicação nativa do JetStream**
//! (janela `duplicate_window`): republicar o mesmo `event_id` dentro
//! da janela não cria uma segunda mensagem na stream — suporte de
//! idempotência no broker (`§15`), sem exigir `EventInbox` local.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use gerax_eda::{Event, EventEnvelope, EventMetadata};
use gerax_es::{EventId, StoredEvent};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::NatsError;

/// Header NATS que carrega o `event_id` (dedup JetStream).
pub const HEADER_MSG_ID: &str = "Nats-Msg-Id";
/// Header NATS de content-type.
pub const HEADER_CONTENT_TYPE: &str = "Content-Type";
/// Header NATS que carrega a causação.
pub const HEADER_CAUSATION_ID: &str = "x-causation-id";
/// Header NATS que carrega a correlação.
pub const HEADER_CORRELATION_ID: &str = "x-correlation-id";
/// Content-type fixo das mensagens do adapter.
pub const CONTENT_TYPE_JSON: &str = "application/json";

/// Envelope de transporte publicado no NATS.
///
/// É a forma *em trânsito* do evento: os campos se adaptam às
/// abstrações reais de `gerax-eda` ([`EventEnvelope`]) e `gerax-es`
/// ([`StoredEvent`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NatsEnvelope {
    /// Identificador do evento (para idempotência — `§15`).
    pub event_id: Option<String>,
    /// Nome lógico estável do evento (usado no subject).
    pub event_type: String,
    /// Identificador do aggregate dono do evento (quando conhecido).
    pub aggregate_id: Option<String>,
    /// Tipo do aggregate dono do evento (quando conhecido).
    pub aggregate_type: Option<String>,
    /// Versão do aggregate nesta stream (quando conhecida).
    pub version: Option<u64>,
    /// Instante em que o evento ocorreu.
    pub occurred_at: DateTime<Utc>,
    /// Identificador de correlação do fluxo, se conhecido.
    pub correlation_id: Option<String>,
    /// Identificador do evento causador, se conhecido.
    pub causation_id: Option<String>,
    /// Headers genéricos de extensão (ex.: propagação de tracing).
    pub headers: HashMap<String, String>,
    /// Payload do evento serializado.
    pub payload: Value,
}

impl NatsEnvelope {
    /// Constrói o envelope de transporte a partir de um envelope EDA.
    pub fn from_event<E>(envelope: &EventEnvelope<E>) -> Result<Self, NatsError>
    where
        E: Event + Serialize,
    {
        let payload = serde_json::to_value(&envelope.event).map_err(|source| {
            NatsError::Serialization(format!("serialize event payload failed: {source}"))
        })?;

        Ok(Self {
            event_id: Some(envelope.id.as_uuid().to_string()),
            event_type: envelope.metadata.event_type.clone(),
            aggregate_id: None,
            aggregate_type: None,
            version: None,
            occurred_at: envelope.metadata.occurred_at,
            correlation_id: envelope
                .metadata
                .correlation_id
                .as_ref()
                .map(ToString::to_string),
            causation_id: envelope
                .metadata
                .causation_id
                .as_ref()
                .map(ToString::to_string),
            headers: envelope.metadata.headers.clone(),
            payload,
        })
    }

    /// Constrói o envelope de transporte a partir de um evento persistido.
    ///
    /// Permite que um *OutboxPublisher* publique entradas de outbox
    /// preservando aggregate e versão.
    pub fn from_stored(event: &StoredEvent) -> Result<Self, NatsError> {
        let mut headers = HashMap::new();
        let mut correlation_id = None;
        let mut causation_id = None;

        for (key, value) in event.metadata.iter() {
            let Some(text) = value.as_str() else {
                continue;
            };
            match key.as_str() {
                "correlation_id" => correlation_id = Some(text.to_string()),
                "causation_id" => causation_id = Some(text.to_string()),
                _ => {
                    headers.insert(key.clone(), text.to_string());
                }
            }
        }

        Ok(Self {
            event_id: Some(event.event_id.as_uuid().to_string()),
            event_type: event.event_type.clone(),
            aggregate_id: Some(event.aggregate_id.clone()),
            aggregate_type: Some(event.aggregate_type.clone()),
            version: Some(event.version.value()),
            occurred_at: event.occurred_at,
            correlation_id,
            causation_id,
            headers,
            payload: event.payload.clone(),
        })
    }

    /// Referência ao `event_type` (token final do subject).
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Identificador do evento; novo UUID quando ausente/inválido.
    pub fn event_id(&self) -> EventId {
        self.event_id
            .as_deref()
            .and_then(|raw| uuid::Uuid::parse_str(raw).ok())
            .map(EventId::from_uuid)
            .unwrap_or_default()
    }

    /// Desserializa o payload em um tipo concreto.
    pub fn payload<E>(&self) -> Result<E, NatsError>
    where
        E: DeserializeOwned,
    {
        serde_json::from_value(self.payload.clone()).map_err(|source| {
            NatsError::Deserialization(format!(
                "deserialize payload into {} failed: {source}",
                std::any::type_name::<E>()
            ))
        })
    }

    /// Réplica do instante de ocorrência.
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// Converte para a metadata do ecossistema EDA.
    pub fn to_eda_metadata(&self) -> EventMetadata {
        let mut metadata = EventMetadata::new(self.event_type.clone());
        metadata.occurred_at = self.occurred_at;
        if let Some(correlation_id) = &self.correlation_id {
            metadata = metadata.with_correlation_id(correlation_id.clone());
        }
        if let Some(causation_id) = &self.causation_id {
            metadata = metadata.with_causation_id(causation_id.clone());
        }
        for (key, value) in &self.headers {
            metadata.insert_header(key, value);
        }
        metadata
    }
}

/// Headers NATS derivados do envelope (produtor usa esta função).
pub(crate) fn nats_headers(envelope: &NatsEnvelope) -> async_nats::HeaderMap {
    let mut headers = async_nats::HeaderMap::new();
    headers.insert(HEADER_CONTENT_TYPE, CONTENT_TYPE_JSON);
    if let Some(event_id) = &envelope.event_id {
        headers.insert(HEADER_MSG_ID, event_id.clone());
    }
    if let Some(correlation_id) = &envelope.correlation_id {
        headers.insert(HEADER_CORRELATION_ID, correlation_id.clone());
    }
    if let Some(causation_id) = &envelope.causation_id {
        headers.insert(HEADER_CAUSATION_ID, causation_id.clone());
    }
    for (key, value) in &envelope.headers {
        headers.insert(format!("x-{key}"), value.clone());
    }
    headers
}

/// Serializa o envelope em bytes de payload (JSON).
pub(crate) fn encode_envelope(envelope: &NatsEnvelope) -> Result<bytes::Bytes, NatsError> {
    let json = serde_json::to_vec(envelope).map_err(|source| {
        NatsError::Serialization(format!("envelope encoding failed: {source}"))
    })?;
    Ok(bytes::Bytes::from(json))
}

/// Desserializa bytes de payload em um envelope.
#[cfg(any(feature = "jetstream", test))]
pub(crate) fn decode_envelope(payload: &[u8]) -> Result<NatsEnvelope, NatsError> {
    serde_json::from_slice(payload)
        .map_err(|source| NatsError::Deserialization(format!("envelope decoding failed: {source}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
        curso: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    fn envelope_eda() -> EventEnvelope<AlunoMatriculado> {
        EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
            curso: "matematica".into(),
        })
        .with_correlation_id("fluxo-42")
        .with_causation_id("evt-anterior")
        .with_header("tenant", "cmf")
    }

    #[test]
    fn envelope_json_roundtrip() {
        let original = NatsEnvelope::from_event(&envelope_eda()).unwrap();
        let bytes = encode_envelope(&original).unwrap();
        let decoded = decode_envelope(&bytes).unwrap();

        assert_eq!(decoded, original);
        assert_eq!(decoded.payload["aluno_id"].as_str(), Some("a-1"));
    }

    #[test]
    fn from_event_carrega_rastreabilidade() {
        let envelope = NatsEnvelope::from_event(&envelope_eda()).unwrap();

        assert_eq!(envelope.event_type(), "aluno.matriculado");
        assert_eq!(envelope.correlation_id.as_deref(), Some("fluxo-42"));
        assert_eq!(envelope.causation_id.as_deref(), Some("evt-anterior"));
        assert_eq!(
            envelope.headers.get("tenant").map(String::as_str),
            Some("cmf")
        );
        assert!(envelope.aggregate_id.is_none());
        assert!(envelope.version.is_none());
    }

    #[test]
    fn payload_e_desserializado_no_tipo_concreto() {
        let envelope = NatsEnvelope::from_event(&envelope_eda()).unwrap();
        let evento = envelope.payload::<AlunoMatriculado>().unwrap();

        assert_eq!(evento.aluno_id, "a-1");
        assert_eq!(evento.curso, "matematica");
    }

    #[test]
    fn payload_invalido_gera_erro_tipado() {
        let mut envelope = NatsEnvelope::from_event(&envelope_eda()).unwrap();
        envelope.payload = Value::Null;

        let err = envelope.payload::<AlunoMatriculado>().unwrap_err();
        assert!(matches!(err, NatsError::Deserialization(_)));
    }

    #[test]
    fn headers_nats_carregam_msg_id_e_rastreabilidade() {
        let envelope = NatsEnvelope::from_event(&envelope_eda()).unwrap();
        let headers = nats_headers(&envelope);

        assert_eq!(
            headers.get(HEADER_CONTENT_TYPE).unwrap().as_str(),
            CONTENT_TYPE_JSON
        );
        assert_eq!(
            headers.get(HEADER_MSG_ID).unwrap().as_str(),
            envelope.event_id.as_deref().unwrap()
        );
        assert_eq!(
            headers.get(HEADER_CORRELATION_ID).unwrap().as_str(),
            "fluxo-42"
        );
        assert_eq!(
            headers.get(HEADER_CAUSATION_ID).unwrap().as_str(),
            "evt-anterior"
        );
        assert_eq!(headers.get("x-tenant").unwrap().as_str(), "cmf");
    }

    #[test]
    fn to_eda_metadata_mapeia_rastreabilidade() {
        let envelope = NatsEnvelope::from_event(&envelope_eda()).unwrap();
        let metadata = envelope.to_eda_metadata();

        assert_eq!(metadata.event_type, "aluno.matriculado");
        assert_eq!(
            metadata.correlation_id.as_ref().map(|c| c.as_str()),
            Some("fluxo-42")
        );
        assert_eq!(metadata.header("tenant"), Some("cmf"));
    }

    #[test]
    fn decode_rejeita_bytes_invalidos() {
        let err = decode_envelope(b"nao-e-json").unwrap_err();
        assert!(matches!(err, NatsError::Deserialization(_)));
    }

    #[test]
    fn from_stored_preserva_aggregate_e_versao() {
        use gerax_es::Version;

        let mut metadata = gerax_es::EventMetadata::new();
        metadata.insert("correlation_id", "fluxo-stored");
        metadata.insert("tenant_id", "ten-1");

        let stored = StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "student-1".into(),
            aggregate_type: "Student".into(),
            event_type: "aluno.matriculado".into(),
            version: Version::new(7),
            payload: serde_json::json!({ "aluno_id": "a-1" }),
            metadata,
            occurred_at: Utc::now(),
        };

        let envelope = NatsEnvelope::from_stored(&stored).unwrap();

        assert_eq!(envelope.aggregate_id.as_deref(), Some("student-1"));
        assert_eq!(envelope.aggregate_type.as_deref(), Some("Student"));
        assert_eq!(envelope.version, Some(7));
        assert_eq!(envelope.correlation_id.as_deref(), Some("fluxo-stored"));
        assert_eq!(
            envelope.headers.get("tenant_id").map(String::as_str),
            Some("ten-1")
        );
    }
}
