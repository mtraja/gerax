//! Envelope de transporte das mensagens (`§7`, `§8`).
//!
//! Nenhum payload de evento é publicado "cru": o [`MessageEnvelope`]
//! empacota identidade, tipo, rastreabilidade (correlation/causation),
//! versionamento e o payload JSON.
//!
//! O formato de serialização fica isolado no codec ([`EnvelopeCodec`]),
//! permitindo futuramente JSON, MessagePack, Protobuf ou Avro sem
//! alterar a semântica do `EventBus` (`§8`).
//!
//! # Convenção de headers AMQP (contrato do adapter)
//!
//! ```text
//! AMQP property / header        MessageEnvelope
//! ─────────────────────────────  ────────────────────────────────
//! contentType                   application/json (fixo)
//! contentEncoding               utf-8 (fixo)
//! appId                         gerax-rabbitmq (fixo)
//! deliveryMode                  conforme RabbitMqConfig
//! messageId                     event_id (UUID)
//! type                          event_type
//! timestamp                     occurred_at (segundos desde epoch)
//! correlationId                 correlation_id
//! headers["x-causation-id"]     causation_id
//! headers["x-gerax-deliveries"] contagem de tentativas (retry)
//! headers["x-<key>"]            headers[<key>]
//! ```

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use gerax_eda::{Event, EventEnvelope, EventMetadata};
use gerax_es::{EventId, StoredEvent};
use lapin::BasicProperties;
use lapin::types::{AMQPValue, FieldTable, ShortString};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::DeliveryMode;
use crate::error::RabbitMqError;

/// Header que carrega a contagem de entregas para a política de retry.
pub(crate) const HEADER_CAUSATION_ID: &str = "x-causation-id";
/// Header (na metadata) que conta as tentativas de processamento.
pub(crate) const HEADER_RETRY_ATTEMPT: &str = "x-gerax-deliveries";
/// `app_id` fixo gravado nas mensagens publicadas.
pub(crate) const APP_ID: &str = "gerax-rabbitmq";

/// Envelope de transporte publicado no RabbitMQ.
///
/// É a forma *em trânsito* do evento: os campos se adaptam às
/// abstrações reais de `gerax-eda` ([`EventEnvelope`]) e `gerax-es`
/// ([`StoredEvent`]). O corpo da mensagem é este envelope serializado
/// em JSON (`application/json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Identificador do evento (para idempotência — `§20`).
    pub event_id: Option<String>,
    /// Nome lógico estável do evento (usado como routing key).
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

impl MessageEnvelope {
    /// Constrói o envelope de transporte a partir de um envelope EDA.
    pub fn from_event<E>(envelope: &EventEnvelope<E>) -> Result<Self, RabbitMqError>
    where
        E: Event + Serialize,
    {
        let payload = serde_json::to_value(&envelope.event).map_err(|source| {
            RabbitMqError::Serialization(format!("serialize event payload failed: {source}"))
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
    /// Permite que um *OutboxPublisher* publique diretamente as
    /// entradas da outbox (`§23`) preservando aggregate e versão.
    pub fn from_stored(event: &StoredEvent) -> Result<Self, RabbitMqError> {
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

    /// Referência ao `event_type` (routing key).
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
    pub fn payload<E>(&self) -> Result<E, RabbitMqError>
    where
        E: DeserializeOwned,
    {
        serde_json::from_value(self.payload.clone()).map_err(|source| {
            RabbitMqError::Deserialization(format!(
                "deserialize payload into {} failed: {source}",
                std::any::type_name::<E>()
            ))
        })
    }

    /// Réplica do instante de ocorrência.
    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    /// Número de tentativas já realizadas (lido de `headers`).
    pub fn attempts(&self) -> u32 {
        self.headers
            .get(HEADER_RETRY_ATTEMPT)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    /// Define a contagem de tentativas na metadata (retry).
    pub fn set_attempts(&mut self, attempts: u32) {
        self.headers
            .insert(HEADER_RETRY_ATTEMPT.to_string(), attempts.to_string());
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
            if key != HEADER_RETRY_ATTEMPT {
                metadata.insert_header(key, value);
            }
        }
        metadata
    }
}

/// Serializa/desserializa o [`MessageEnvelope`].
///
/// Isola o formato da mensagem no wire (`§8`): o `EventBus` não conhece
/// o codec, apenas o envelope.
pub trait EnvelopeCodec: Clone + Send + Sync + 'static {
    /// Serializa o envelope em bytes.
    fn encode(&self, envelope: &MessageEnvelope) -> Result<Vec<u8>, RabbitMqError>;

    /// Desserializa bytes em um envelope.
    fn decode(&self, bytes: &[u8]) -> Result<MessageEnvelope, RabbitMqError>;
}

/// Codec JSON (padrão da primeira versão).
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonEnvelopeCodec;

impl EnvelopeCodec for JsonEnvelopeCodec {
    fn encode(&self, envelope: &MessageEnvelope) -> Result<Vec<u8>, RabbitMqError> {
        serde_json::to_vec(envelope).map_err(|source| {
            RabbitMqError::Serialization(format!("envelope encoding failed: {source}"))
        })
    }

    fn decode(&self, bytes: &[u8]) -> Result<MessageEnvelope, RabbitMqError> {
        serde_json::from_slice(bytes).map_err(|source| {
            RabbitMqError::Deserialization(format!("envelope decoding failed: {source}"))
        })
    }
}

/// Propriedades AMQP derivadas do envelope para a publicação
/// (produtor e republish de retry usam esta mesma função).
pub(crate) fn amqp_properties(
    envelope: &MessageEnvelope,
    delivery_mode: DeliveryMode,
) -> BasicProperties {
    let mut properties = BasicProperties::default()
        .with_app_id(ShortString::from(APP_ID))
        .with_content_type(ShortString::from("application/json"))
        .with_content_encoding(ShortString::from("utf-8"))
        .with_delivery_mode(match delivery_mode {
            DeliveryMode::Persistent => 2,
            DeliveryMode::Transient => 1,
        })
        .with_message_id(ShortString::from(envelope.event_id().as_uuid().to_string()))
        .with_type(ShortString::from(envelope.event_type.clone()))
        .with_timestamp(envelope.occurred_at.timestamp().max(0) as u64);

    if let Some(correlation_id) = &envelope.correlation_id {
        properties = properties.with_correlation_id(ShortString::from(correlation_id.clone()));
    }

    let mut headers = FieldTable::default();
    if let Some(causation_id) = &envelope.causation_id {
        headers.insert(
            HEADER_CAUSATION_ID.into(),
            AMQPValue::LongString(causation_id.clone().into()),
        );
    }
    for (key, value) in &envelope.headers {
        headers.insert(
            format!("x-{key}").into(),
            AMQPValue::LongString(value.clone().into()),
        );
    }

    if !headers.inner().is_empty() {
        properties = properties.with_headers(headers);
    }

    properties
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_eda::CorrelationId;
    use gerax_es::Version;

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
        let original = MessageEnvelope::from_event(&envelope_eda()).unwrap();
        let bytes = JsonEnvelopeCodec.encode(&original).unwrap();
        let decoded = JsonEnvelopeCodec.decode(&bytes).unwrap();

        assert_eq!(decoded, original);
        assert_eq!(decoded.payload["aluno_id"].as_str(), Some("a-1"));
    }

    #[test]
    fn from_event_carrega_rastreabilidade() {
        let envelope = MessageEnvelope::from_event(&envelope_eda()).unwrap();

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
        let envelope = MessageEnvelope::from_event(&envelope_eda()).unwrap();
        let evento = envelope.payload::<AlunoMatriculado>().unwrap();

        assert_eq!(evento.aluno_id, "a-1");
        assert_eq!(evento.curso, "matematica");
    }

    #[test]
    fn payload_invalido_gera_erro_tipado() {
        let mut envelope = MessageEnvelope::from_event(&envelope_eda()).unwrap();
        envelope.payload = Value::Null;

        let err = envelope.payload::<AlunoMatriculado>().unwrap_err();
        assert!(matches!(err, RabbitMqError::Deserialization(_)));
    }

    #[test]
    fn event_id_invalido_gera_novo_uuid() {
        let mut envelope = MessageEnvelope::from_event(&envelope_eda()).unwrap();
        let original = envelope.event_id();
        assert_eq!(envelope.event_id(), original);

        envelope.event_id = Some("nao-e-uuid".into());
        let fallback = envelope.event_id();
        assert_ne!(fallback, original);
        assert_eq!(
            fallback.as_uuid().get_version(),
            Some(uuid::Version::Random)
        );
    }

    #[test]
    fn attempts_sao_contabilizados_na_metadata() {
        let mut envelope = MessageEnvelope::from_event(&envelope_eda()).unwrap();
        assert_eq!(envelope.attempts(), 0);

        envelope.set_attempts(3);
        assert_eq!(envelope.attempts(), 3);
    }

    #[test]
    fn from_stored_preserva_aggregate_e_versao() {
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

        let envelope = MessageEnvelope::from_stored(&stored).unwrap();

        assert_eq!(envelope.aggregate_id.as_deref(), Some("student-1"));
        assert_eq!(envelope.aggregate_type.as_deref(), Some("Student"));
        assert_eq!(envelope.version, Some(7));
        assert_eq!(envelope.correlation_id.as_deref(), Some("fluxo-stored"));
        assert_eq!(
            envelope.headers.get("tenant_id").map(String::as_str),
            Some("ten-1")
        );
    }

    #[test]
    fn amqp_properties_carregam_identidade() {
        let envelope = MessageEnvelope::from_event(&envelope_eda()).unwrap();
        let properties = amqp_properties(&envelope, DeliveryMode::Persistent);

        assert_eq!(
            properties.content_type().as_ref().map(ToString::to_string),
            Some("application/json".into())
        );
        assert_eq!(
            properties.app_id().as_ref().map(ToString::to_string),
            Some("gerax-rabbitmq".into())
        );
        assert_eq!(properties.delivery_mode(), &Some(2));
        assert_eq!(
            properties.kind().as_ref().map(ToString::to_string),
            Some("aluno.matriculado".into())
        );
        assert_eq!(
            properties
                .correlation_id()
                .as_ref()
                .map(ToString::to_string),
            Some("fluxo-42".into())
        );
    }

    #[test]
    fn to_eda_metadata_mapeia_rastreabilidade() {
        let envelope = MessageEnvelope::from_event(&envelope_eda()).unwrap();
        let metadata = envelope.to_eda_metadata();

        assert_eq!(metadata.event_type, "aluno.matriculado");
        assert_eq!(
            metadata.correlation_id.as_ref().map(CorrelationId::as_str),
            Some("fluxo-42")
        );
        assert_eq!(metadata.header("tenant"), Some("cmf"));
        assert_eq!(metadata.header(HEADER_RETRY_ATTEMPT), None);
    }

    #[test]
    fn codec_rejeita_bytes_invalidos() {
        let err = JsonEnvelopeCodec.decode(b"nao-e-json").unwrap_err();
        assert!(matches!(err, RabbitMqError::Deserialization(_)));
    }
}
