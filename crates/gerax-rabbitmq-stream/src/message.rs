//! Envelope do evento e sua tradução para a mensagem de streams
//! (§10–§12).
//!
//! Nenhum payload é publicado "cru". O [`MessageEnvelope`] é a forma
//! *em trânsito* do evento e se materializa em **duas camadas** do
//! protocolo de streams, cada uma com um propósito:
//!
//! ```text
//! MessageEnvelope
//!   │
//!   ├── body                    EventSerializer → bytes do payload
//!   │                          (§10 — "Message.body = JSON bytes")
//!   │
//!   └── application_properties  event_type, event_id, aggregate_*,
//!                              correlation_id, causation_id,
//!                              content_type, schema_version,
//!                              occurred_at, headers (§12)
//! ```
//!
//! Por que dividir em vez de embutir tudo num único JSON:
//!
//! - **correlação e causação via properties**: o
//!   `application_properties` do RabbitMQ Streams é indexado pelo
//!   broker, o que habilita o *filtering* server-side sem mudança
//!   no `EventBus` (§28);
//! - **correlação com `gerax-eda`**: o
//!   [`gerax_eda::EventEnvelope`] é reconstruído a partir das
//!   properties + body, sem que o handler conheça
//!   `rabbitmq_stream_client` (§17);
//! - **formatos futuros**: trocar o [`EventSerializer`] por
//!   MessagePack/bincode/Protobuf muda apenas o `body` — as
//!   properties continuam texto legível para o broker (§10).
//!
//! O envelope **não** é acoplado ao domínio: ele é derivado de
//! [`gerax_eda::EventEnvelope`], não o contrário.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use gerax_eda::EventId;
use gerax_eda::{Event, EventEnvelope as EdaEnvelope, EventMetadata};
use rabbitmq_stream_client::types::{Delivery, Message, SimpleValue};
use serde::Serialize;
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::error::RabbitMqStreamError;

/// `application_property` que carrega o `event_type`.
pub const PROP_EVENT_TYPE: &str = "event_type";
/// `application_property` que carrega o `event_id`.
pub const PROP_EVENT_ID: &str = "event_id";
/// `application_property` que carrega o `aggregate_id`.
pub const PROP_AGGREGATE_ID: &str = "aggregate_id";
/// `application_property` que carrega o `aggregate_type`.
pub const PROP_AGGREGATE_TYPE: &str = "aggregate_type";
/// `application_property` que carrega a versão do aggregate.
pub const PROP_VERSION: &str = "version";
/// `application_property` que carrega o instante de ocorrência
/// (RFC 3339).
pub const PROP_OCCURRED_AT: &str = "occurred_at";
/// `application_property` que carrega o `correlation_id`.
pub const PROP_CORRELATION_ID: &str = "correlation_id";
/// `application_property` que carrega o `causation_id`.
pub const PROP_CAUSATION_ID: &str = "causation_id";
/// `application_property` que carrega o `content_type` do payload.
pub const PROP_CONTENT_TYPE: &str = "content_type";
/// `application_property` que carrega a versão do schema do
/// envelope.
pub const PROP_SCHEMA_VERSION: &str = "schema_version";
/// Prefixo das `application_properties` derivadas dos headers do
/// envelope do `gerax-eda`.
pub const PROP_HEADER_PREFIX: &str = "x-";

/// Versão do formato do envelope.
pub const ENVELOPE_SCHEMA_VERSION: u32 = 1;

/// Serializa o **payload** do evento (§10).
///
/// O `gerax-eda` não conhece formato de serialização; o adapter o
/// escolhe e o substitui sem alterar o `EventBus`. A primeira
/// versão usa [`JsonEventSerializer`] (`serde` + `serde_json`).
///
/// ```text
/// Message.body = EventSerializer::encode(event) → bytes
/// event        = EventSerializer::decode(body)  → evento
/// ```
pub trait EventSerializer: Send + Sync {
    /// MIME type do payload (gravado em `content_type`).
    fn content_type(&self) -> &str;

    /// Serializa o payload do evento.
    fn encode<E>(&self, event: &E) -> Result<Vec<u8>, RabbitMqStreamError>
    where
        E: Event + Serialize;

    /// Desserializa o payload do evento.
    fn decode<E>(&self, bytes: &[u8]) -> Result<E, RabbitMqStreamError>
    where
        E: DeserializeOwned;
}

/// [`EventSerializer`] JSON — padrão da primeira versão (§10).
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonEventSerializer;

impl EventSerializer for JsonEventSerializer {
    fn content_type(&self) -> &str {
        "application/json"
    }

    fn encode<E>(&self, event: &E) -> Result<Vec<u8>, RabbitMqStreamError>
    where
        E: Event + Serialize,
    {
        serde_json::to_vec(event).map_err(|source| {
            RabbitMqStreamError::serialization(format!(
                "serialize {} failed: {source}",
                std::any::type_name::<E>()
            ))
        })
    }

    fn decode<E>(&self, bytes: &[u8]) -> Result<E, RabbitMqStreamError>
    where
        E: DeserializeOwned,
    {
        serde_json::from_slice(bytes).map_err(|source| {
            RabbitMqStreamError::serialization(format!(
                "deserialize {} failed: {source}",
                std::any::type_name::<E>()
            ))
        })
    }
}

/// Envelope de transporte publicado no stream (§11).
///
/// ```text
/// MessageEnvelope
///  ├── event_id        EventId (uuid) — idempotência do handler
///  ├── event_type      nome lógico estável (roteamento/filtering)
///  ├── aggregate_id    aggregate dono do evento (quando conhecido)
///  ├── aggregate_type
///  ├── version         versão do aggregate (quando conhecido)
///  ├── occurred_at     instante do fato
///  ├── correlation_id  fluxo de processamento
///  ├── causation_id    evento que originou este
///  ├── headers         extensão (string → string)
///  └── payload         bytes produced por EventSerializer
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEnvelope {
    /// Identificador único do evento.
    pub event_id: Uuid,
    /// Nome lógico estável do evento.
    pub event_type: String,
    /// Aggregate dono do evento, quando conhecido.
    pub aggregate_id: Option<String>,
    /// Tipo do aggregate, quando conhecido.
    pub aggregate_type: Option<String>,
    /// Versão do aggregate, quando conhecida.
    pub version: Option<u64>,
    /// Instante em que o fato ocorreu.
    pub occurred_at: DateTime<Utc>,
    /// Correlação do fluxo de processamento.
    ///
    /// `String` (e não `Uuid`): o `gerax-eda` modela
    /// `CorrelationId`/`CausationId` como identificadores textuais
    /// livres — exigir UUID aqui perderia correlações reais.
    pub correlation_id: Option<String>,
    /// Evento que causou este.
    pub causation_id: Option<String>,
    /// Headers genéricos (propagação de tracing, tenant, ...).
    pub headers: HashMap<String, String>,
    /// Payload serializado.
    pub payload: Vec<u8>,
}

impl MessageEnvelope {
    /// Deriva o envelope de transporte de um envelope do `gerax-eda`.
    ///
    /// O `event_id` do `gerax-eda` ([`gerax_eda::EventId`]) é
    /// preservado: é ele que permite a um consumidor idempotente
    /// reconhecer reprocessamentos (§24).
    pub fn from_eda<E, S>(
        envelope: &EdaEnvelope<E>,
        serializer: &S,
    ) -> Result<Self, RabbitMqStreamError>
    where
        E: Event + Serialize,
        S: EventSerializer,
    {
        Ok(Self {
            event_id: envelope.id.as_uuid(),
            event_type: envelope.metadata.event_type.clone(),
            aggregate_id: None,
            aggregate_type: None,
            version: None,
            occurred_at: envelope.metadata.occurred_at,
            correlation_id: envelope
                .metadata
                .correlation_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            causation_id: envelope
                .metadata
                .causation_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            headers: envelope.metadata.headers.clone(),
            payload: serializer.encode(&envelope.event)?,
        })
    }

    /// Constrói um envelope diretamente (sem passar pelo
    /// `gerax-eda`) — útil em testes e integrações de baixo nível.
    pub fn new(event_type: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type: event_type.into(),
            aggregate_id: None,
            aggregate_type: None,
            version: None,
            occurred_at: Utc::now(),
            correlation_id: None,
            causation_id: None,
            headers: HashMap::new(),
            payload,
        }
    }

    /// Define o aggregate dono do evento (builder).
    pub fn with_aggregate(
        mut self,
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
    ) -> Self {
        self.aggregate_id = Some(aggregate_id.into());
        self.aggregate_type = Some(aggregate_type.into());
        self
    }

    /// Define a versão do aggregate (builder).
    pub fn with_version(mut self, version: u64) -> Self {
        self.version = Some(version);
        self
    }

    /// Insere um header (builder).
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Define a correlação (builder).
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Define a causação (builder).
    pub fn with_causation_id(mut self, id: impl Into<String>) -> Self {
        self.causation_id = Some(id.into());
        self
    }

    /// Reconstrói o envelope do `gerax-eda` com o evento desserializado.
    ///
    /// A metadata é a do `gerax-eda` traduzida de volta das
    /// properties: o handler recebe `EventEnvelope<E>` puro e não
    /// conhece o transporte (§17).
    pub fn to_eda<E, S>(&self, serializer: &S) -> Result<EdaEnvelope<E>, RabbitMqStreamError>
    where
        E: Event + DeserializeOwned,
        S: EventSerializer,
    {
        let event: E = serializer.decode(&self.payload)?;

        // O `event_type` gravado no envelope precisa bater com o
        // contrato do tipo: caso contrário o roteamento silenciaria
        // uma mensagem incompatível.
        if event.event_type() != self.event_type {
            return Err(RabbitMqStreamError::serialization(format!(
                "envelope declares event_type `{}` but payload is `{}`",
                self.event_type,
                event.event_type()
            )));
        }

        let mut metadata = EventMetadata::new(self.event_type.clone());
        metadata.occurred_at = self.occurred_at;
        if let Some(correlation_id) = &self.correlation_id {
            metadata = metadata.with_correlation_id(correlation_id.clone());
        }
        if let Some(causation_id) = &self.causation_id {
            metadata = metadata.with_causation_id(causation_id.clone());
        }
        for (key, value) in &self.headers {
            metadata.insert_header(key.clone(), value.clone());
        }

        Ok(EdaEnvelope {
            id: EventId::from_uuid(self.event_id),
            event,
            metadata,
        })
    }

    /// Converte o envelope em uma mensagem do protocolo de streams.
    ///
    /// `body` recebe o payload; as properties recebem a
    /// rastreabilidade (§12).
    pub(crate) fn to_message<S>(&self, serializer: &S) -> Message
    where
        S: EventSerializer,
    {
        let event_id = self.event_id.to_string();
        let occurred_at = self.occurred_at.to_rfc3339();
        let schema_version = ENVELOPE_SCHEMA_VERSION.to_string();

        let mut builder = Message::builder().body(self.payload.clone());

        builder = builder
            .application_properties()
            .insert(PROP_EVENT_TYPE, self.event_type.as_str())
            .insert(PROP_EVENT_ID, event_id.as_str())
            .insert(PROP_OCCURRED_AT, occurred_at.as_str())
            .insert(PROP_CONTENT_TYPE, serializer.content_type())
            .insert(PROP_SCHEMA_VERSION, schema_version.as_str())
            .message_builder();

        if let Some(aggregate_id) = &self.aggregate_id {
            builder = builder
                .application_properties()
                .insert(PROP_AGGREGATE_ID, aggregate_id.as_str())
                .message_builder();
        }
        if let Some(aggregate_type) = &self.aggregate_type {
            builder = builder
                .application_properties()
                .insert(PROP_AGGREGATE_TYPE, aggregate_type.as_str())
                .message_builder();
        }
        if let Some(version) = self.version {
            let version = version.to_string();
            builder = builder
                .application_properties()
                .insert(PROP_VERSION, version.as_str())
                .message_builder();
        }
        if let Some(correlation_id) = &self.correlation_id {
            let correlation_id = correlation_id.clone();
            builder = builder
                .application_properties()
                .insert(PROP_CORRELATION_ID, correlation_id.as_str())
                .message_builder();
        }
        if let Some(causation_id) = &self.causation_id {
            let causation_id = causation_id.clone();
            builder = builder
                .application_properties()
                .insert(PROP_CAUSATION_ID, causation_id.as_str())
                .message_builder();
        }
        for (key, value) in &self.headers {
            builder = builder
                .application_properties()
                .insert(format!("{PROP_HEADER_PREFIX}{key}"), value.as_str())
                .message_builder();
        }

        builder.build()
    }

    /// Reconstroi o envelope a partir de uma entrega do protocolo.
    pub(crate) fn from_delivery<S>(
        delivery: &Delivery,
        serializer: &S,
    ) -> Result<Self, RabbitMqStreamError>
    where
        S: EventSerializer,
    {
        let payload = delivery.message().data().ok_or_else(|| {
            RabbitMqStreamError::serialization(format!(
                "message on stream `{}` has no data section",
                delivery.stream()
            ))
        })?;

        let properties = application_properties(delivery.message());

        let event_type = required_property(&properties, PROP_EVENT_TYPE)?;
        let event_id = required_property(&properties, PROP_EVENT_ID)?
            .parse::<Uuid>()
            .map_err(|source| {
                RabbitMqStreamError::serialization(format!(
                    "invalid `{PROP_EVENT_ID}` in application properties: {source}"
                ))
            })?;
        let occurred_at = required_property(&properties, PROP_OCCURRED_AT)?
            .parse::<DateTime<Utc>>()
            .map_err(|source| {
                RabbitMqStreamError::serialization(format!(
                    "invalid `{PROP_OCCURRED_AT}` in application properties: {source}"
                ))
            })?;

        let content_type = properties.get(PROP_CONTENT_TYPE).map(String::as_str);
        if let Some(content_type) = content_type
            && content_type != serializer.content_type()
        {
            return Err(RabbitMqStreamError::serialization(format!(
                "unexpected content_type `{content_type}` (serializer expects `{}`)",
                serializer.content_type()
            )));
        }

        let mut headers = HashMap::new();
        for (key, value) in &properties {
            if let Some(name) = key.strip_prefix(PROP_HEADER_PREFIX) {
                headers.insert(name.to_string(), value.clone());
            }
        }

        Ok(Self {
            event_id,
            event_type,
            aggregate_id: properties.get(PROP_AGGREGATE_ID).cloned(),
            aggregate_type: properties.get(PROP_AGGREGATE_TYPE).cloned(),
            version: properties
                .get(PROP_VERSION)
                .and_then(|value| value.parse().ok()),
            occurred_at,
            correlation_id: properties.get(PROP_CORRELATION_ID).cloned(),
            causation_id: properties.get(PROP_CAUSATION_ID).cloned(),
            headers,
            payload: payload.to_vec(),
        })
    }
}

/// Lê as `application_properties` da mensagem como mapa de strings.
///
/// O protocolo define `Map<String, SimpleValue>`; o adapter só grava
/// strings, então qualquer outro tipo é ruido e é ignorado.
fn application_properties(message: &Message) -> HashMap<String, String> {
    let Some(properties) = message.application_properties() else {
        return HashMap::new();
    };

    properties
        .iter()
        .filter_map(|(key, value)| match value {
            SimpleValue::String(text) => Some((key.clone(), text.clone())),
            _ => None,
        })
        .collect()
}

/// Property obrigatória do envelope, com mensagem de erro útil.
fn required_property(
    properties: &HashMap<String, String>,
    key: &str,
) -> Result<String, RabbitMqStreamError> {
    properties
        .get(key)
        .cloned()
        .ok_or_else(|| RabbitMqStreamError::serialization(format!(
            "missing `{key}` in application properties (message not published by gerax-rabbitmq-stream?)"
        )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_eda::{EventHandler, EventHandlerError};

    #[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
        curso: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    fn eda_envelope() -> EdaEnvelope<AlunoMatriculado> {
        EdaEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
            curso: "matematica".into(),
        })
        .with_correlation_id("1f0d4a9e-0000-4000-8000-000000000001")
        .with_causation_id("1f0d4a9e-0000-4000-8000-000000000002")
        .with_header("tenant", "cmf")
    }

    #[test]
    fn serializer_json_roundtrip() {
        let event = AlunoMatriculado {
            aluno_id: "a-1".into(),
            curso: "fisica".into(),
        };

        let bytes = JsonEventSerializer.encode(&event).expect("encode");
        let back: AlunoMatriculado = JsonEventSerializer.decode(&bytes).expect("decode");

        assert_eq!(back, event);
        assert_eq!(JsonEventSerializer.content_type(), "application/json");
    }

    #[test]
    fn serializer_rejeita_payload_invalido() {
        let err = JsonEventSerializer
            .decode::<AlunoMatriculado>(b"nao-e-json")
            .expect_err("payload invalido");
        assert!(matches!(err, RabbitMqStreamError::Serialization(_)));
    }

    #[test]
    fn from_eda_preserva_identidade_e_rastreabilidade() {
        let eda = eda_envelope();
        let envelope = MessageEnvelope::from_eda(&eda, &JsonEventSerializer).expect("envelope");

        assert_eq!(envelope.event_id, eda.id.as_uuid());
        assert_eq!(envelope.event_type, "aluno.matriculado");
        assert_eq!(envelope.occurred_at, eda.metadata.occurred_at);
        assert_eq!(
            envelope.correlation_id,
            Some("1f0d4a9e-0000-4000-8000-000000000001".to_string())
        );
        assert_eq!(
            envelope.causation_id,
            Some("1f0d4a9e-0000-4000-8000-000000000002".to_string())
        );
        assert_eq!(
            envelope.headers.get("tenant").map(String::as_str),
            Some("cmf")
        );
    }

    #[test]
    fn payload_do_envelope_e_o_json_do_evento() {
        let envelope =
            MessageEnvelope::from_eda(&eda_envelope(), &JsonEventSerializer).expect("envelope");
        let value: serde_json::Value = serde_json::from_slice(&envelope.payload).expect("json");

        assert_eq!(value["aluno_id"], "a-1");
        assert_eq!(value["curso"], "matematica");
    }

    #[test]
    fn to_eda_reconstroi_o_envelope_do_gerax_eda() {
        let original = eda_envelope();
        let envelope =
            MessageEnvelope::from_eda(&original, &JsonEventSerializer).expect("envelope");
        let back: EdaEnvelope<AlunoMatriculado> =
            envelope.to_eda(&JsonEventSerializer).expect("eda");

        assert_eq!(back.id, original.id);
        assert_eq!(back.event, original.event);
        assert_eq!(back.metadata.event_type, original.metadata.event_type);
        assert_eq!(back.metadata.occurred_at, original.metadata.occurred_at);
        assert_eq!(
            back.metadata.correlation_id.as_ref().map(|c| c.as_str()),
            original
                .metadata
                .correlation_id
                .as_ref()
                .map(|c| c.as_str())
        );
        assert_eq!(back.metadata.header("tenant"), Some("cmf"));
    }

    #[test]
    fn to_eda_rejeita_event_type_divergente() {
        let mut envelope =
            MessageEnvelope::from_eda(&eda_envelope(), &JsonEventSerializer).expect("envelope");
        envelope.event_type = "aluno.cancelado".into();

        let err = envelope
            .to_eda::<AlunoMatriculado, _>(&JsonEventSerializer)
            .expect_err("event_type divergente");
        assert!(err.to_string().contains("aluno.cancelado"));
        assert!(err.to_string().contains("aluno.matriculado"));
    }

    #[test]
    fn to_eda_rejeita_payload_de_outro_tipo() {
        let envelope = MessageEnvelope::new("aluno.matriculado", b"{\"outro\":1}".to_vec());
        assert!(
            envelope
                .to_eda::<AlunoMatriculado, _>(&JsonEventSerializer)
                .is_err()
        );
    }

    #[test]
    fn message_carrega_payload_no_body_e_rastreabilidade_nas_properties() {
        let envelope = MessageEnvelope::from_eda(&eda_envelope(), &JsonEventSerializer)
            .expect("envelope")
            .with_aggregate("a-1", "Aluno")
            .with_version(7);

        let message = envelope.to_message(&JsonEventSerializer);

        assert!(message.data().is_some());
        let properties = application_properties(&message);
        assert_eq!(
            properties.get(PROP_EVENT_TYPE).map(String::as_str),
            Some("aluno.matriculado")
        );
        assert_eq!(
            properties.get(PROP_EVENT_ID).map(String::as_str),
            Some(envelope.event_id.to_string().as_str())
        );
        assert_eq!(
            properties.get(PROP_AGGREGATE_ID).map(String::as_str),
            Some("a-1")
        );
        assert_eq!(
            properties.get(PROP_AGGREGATE_TYPE).map(String::as_str),
            Some("Aluno")
        );
        assert_eq!(properties.get(PROP_VERSION).map(String::as_str), Some("7"));
        assert_eq!(
            properties.get(PROP_CONTENT_TYPE).map(String::as_str),
            Some("application/json")
        );
        assert_eq!(
            properties.get(PROP_SCHEMA_VERSION).map(String::as_str),
            Some("1")
        );
        assert_eq!(properties.get("x-tenant").map(String::as_str), Some("cmf"));
    }

    #[test]
    fn message_e_deterministica_para_o_mesmo_envelope() {
        let envelope =
            MessageEnvelope::from_eda(&eda_envelope(), &JsonEventSerializer).expect("envelope");
        assert_eq!(
            envelope.to_message(&JsonEventSerializer),
            envelope.to_message(&JsonEventSerializer)
        );
    }

    #[test]
    fn envelope_novo_tem_id_unico_e_payload_cru() {
        let a = MessageEnvelope::new("aluno.matriculado", b"{}".to_vec());
        let b = MessageEnvelope::new("aluno.matriculado", b"{}".to_vec());

        assert_ne!(a.event_id, b.event_id);
        assert_eq!(a.payload, b"{}");
        assert!(a.aggregate_id.is_none());
        assert!(a.version.is_none());
    }

    #[tokio::test]
    async fn handler_recebe_envelope_eda_sem_conhecer_o_transporte() {
        // O handler só conhece `gerax_eda` — nenhum tipo de
        // `rabbitmq_stream_client` atravessa a fronteira (§17).
        struct Registrar;
        #[async_trait::async_trait]
        impl EventHandler<AlunoMatriculado> for Registrar {
            async fn handle(
                &self,
                event: EdaEnvelope<AlunoMatriculado>,
            ) -> Result<(), EventHandlerError> {
                assert_eq!(event.event.aluno_id, "a-1");
                Ok(())
            }
        }

        let envelope =
            MessageEnvelope::from_eda(&eda_envelope(), &JsonEventSerializer).expect("envelope");
        let eda: EdaEnvelope<AlunoMatriculado> =
            envelope.to_eda(&JsonEventSerializer).expect("eda");

        Registrar.handle(eda).await.expect("handler");
    }

    #[test]
    fn correlacao_textual_nao_uuid_sobrevive_ao_round_trip() {
        let eda = eda_envelope()
            .with_correlation_id("fluxo-42")
            .with_causation_id("cmd-7");

        let envelope = MessageEnvelope::from_eda(&eda, &JsonEventSerializer).expect("envelope");
        assert_eq!(envelope.correlation_id.as_deref(), Some("fluxo-42"));
        assert_eq!(envelope.causation_id.as_deref(), Some("cmd-7"));

        let back: EdaEnvelope<AlunoMatriculado> =
            envelope.to_eda(&JsonEventSerializer).expect("eda");
        assert_eq!(
            back.metadata.correlation_id.as_ref().map(|id| id.as_str()),
            Some("fluxo-42")
        );
        assert_eq!(
            back.metadata.causation_id.as_ref().map(|id| id.as_str()),
            Some("cmd-7")
        );
    }

    #[test]
    fn required_property_explica_a_ausencia() {
        let err = required_property(&HashMap::new(), PROP_EVENT_TYPE).expect_err("ausente");
        assert!(err.to_string().contains(PROP_EVENT_TYPE));
    }
}
