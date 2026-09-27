//! Tradução entre [`gerax_eda::EventMetadata`] e as propriedades AMQP.
//!
//! Convenção de mapeamento (contrato do adapter):
//!
//! ```text
//! AMQP property              EventMetadata
//! ─────────────────────────  ─────────────────────────────
//! message_id                 EventEnvelope.id (UUID)
//! type                       metadata.event_type
//! timestamp                  metadata.occurred_at (UNIX, segundos)
//! correlation_id             metadata.correlation_id
//! headers["x-causation-id"]  metadata.causation_id
//! headers["x-<key>"]         metadata.headers[<key>]
//! app_id                     "gerax-eda-rabbitmq" (fixo)
//! content_type               application/json    (fixo)
//! ```
//!
//! Nota: a propriedade AMQP `timestamp` tem precisão de segundos; a
//! fração de segundo de `occurred_at` não sobrevive ao round-trip.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use gerax_eda::{CausationId, CorrelationId, Event, EventEnvelope, EventMetadata};
use gerax_es::EventId;
use lapin::BasicProperties;
use lapin::types::{AMQPValue, FieldTable, ShortString};

/// Header que carrega o `causation_id` (reservado pelo adapter).
pub(crate) const HEADER_CAUSATION_ID: &str = "x-causation-id";
/// `app_id` fixo gravado nas mensagens publicadas.
pub(crate) const APP_ID: &str = "gerax-eda-rabbitmq";

/// Constrói as propriedades AMQP a partir do envelope.
///
/// O payload **não** entra aqui: as propriedades descrevem a metadata
/// e o corpo é serializado separadamente pelo codec.
pub(crate) fn properties_from_envelope<E: Event>(envelope: &EventEnvelope<E>) -> BasicProperties {
    let mut headers = FieldTable::default();
    if let Some(causation) = &envelope.metadata.causation_id {
        headers.insert(
            HEADER_CAUSATION_ID.into(),
            AMQPValue::LongString(causation.to_string().into()),
        );
    }
    for (key, value) in &envelope.metadata.headers {
        headers.insert(
            format!("x-{key}").into(),
            AMQPValue::LongString(value.clone().into()),
        );
    }

    let mut properties = BasicProperties::default()
        .with_app_id(APP_ID.into())
        .with_content_type("application/json".into())
        .with_content_encoding("utf-8".into())
        .with_message_id(envelope.id.as_uuid().to_string().into())
        .with_type(ShortString::from(envelope.metadata.event_type.as_str()))
        .with_timestamp(envelope.metadata.occurred_at.timestamp() as u64);

    if let Some(correlation) = &envelope.metadata.correlation_id {
        properties = properties.with_correlation_id(ShortString::from(correlation.as_str()));
    }
    if !headers.inner().is_empty() {
        properties = properties.with_headers(headers);
    }
    properties
}

/// Reconstrói o envelope a partir das propriedades de uma mensagem
/// recebida e do payload já desserializado.
///
/// Campos ausentes recebem valores conservadores: novo `EventId` se
/// `message_id` não for um UUID, `occurred_at = agora`, correlação e
/// causação ausentes.
pub(crate) fn envelope_from_properties<E: Event>(
    properties: &BasicProperties,
    payload: E,
    fallback_event_type: &str,
) -> EventEnvelope<E> {
    let id = match properties.message_id() {
        Some(message_id) => uuid::Uuid::parse_str(message_id.as_str())
            .ok()
            .map(EventId::from_uuid),
        None => None,
    }
    .unwrap_or_else(EventId::new);

    let event_type = match properties.kind() {
        Some(kind) => kind.to_string(),
        None => fallback_event_type.to_string(),
    };

    let occurred_at = match properties.timestamp() {
        Some(timestamp) => DateTime::<Utc>::from_timestamp(timestamp.to_owned() as i64, 0),
        None => None,
    }
    .unwrap_or_else(Utc::now);

    let correlation_id = properties
        .correlation_id()
        .as_ref()
        .map(|correlation_id| CorrelationId::new(correlation_id.as_str()));

    let mut headers = HashMap::new();
    let mut causation_id = None;
    if let Some(headers_table) = properties.headers() {
        for (key, value) in headers_table {
            let Some(text) = amqp_value_as_string(value) else {
                continue;
            };
            if key.as_str() == HEADER_CAUSATION_ID {
                causation_id = Some(CausationId::new(text));
            } else if let Some(name) = key.as_str().strip_prefix("x-") {
                headers.insert(name.to_string(), text);
            }
        }
    }

    EventEnvelope {
        id,
        event: payload,
        metadata: EventMetadata {
            event_type,
            occurred_at,
            correlation_id,
            causation_id,
            headers,
        },
    }
}

/// Conta quantas redeliveries a mensagem já sofreu, lendo o header
/// `x-death` que o RabbitMQ anexa a mensagens reenfileiradas.
///
/// Retorna a soma dos campos `count` de cada entrada; `0` quando o
/// header não existe (primeira entrega).
pub(crate) fn redelivery_count(properties: &BasicProperties) -> u32 {
    let Some(headers) = properties.headers() else {
        return 0;
    };
    let Some(AMQPValue::FieldArray(deaths)) = headers.inner().get("x-death") else {
        return 0;
    };

    let mut total = 0u32;
    for death in deaths.as_slice() {
        let AMQPValue::FieldTable(entry) = death else {
            continue;
        };
        let Some(count) = entry.inner().get("count") else {
            continue;
        };
        let value = match count {
            AMQPValue::LongLongInt(value) => (*value).max(0) as u64,
            AMQPValue::LongInt(value) => (*value).max(0) as u64,
            AMQPValue::ShortInt(value) => (*value).max(0) as u64,
            AMQPValue::LongUInt(value) => *value as u64,
            _ => continue,
        };
        total = total.saturating_add(value as u32);
    }
    total
}

/// Extrai o texto de um valor AMQP do tipo string.
fn amqp_value_as_string(value: &AMQPValue) -> Option<String> {
    match value {
        AMQPValue::LongString(value) => Some(value.to_string()),
        AMQPValue::ShortString(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde::Serialize;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    fn envelope() -> EventEnvelope<AlunoMatriculado> {
        EventEnvelope::new(AlunoMatriculado {
            aluno_id: "a-1".into(),
        })
        .with_correlation_id("fluxo-1")
        .with_causation_id("evt-0")
        .with_header("tenant", "cmf")
    }

    #[test]
    fn properties_preservam_metadata_no_roundtrip() {
        let original = envelope();
        let properties = properties_from_envelope(&original);

        let rebuilt = envelope_from_properties(&properties, original.event.clone(), "fallback");

        assert_eq!(rebuilt.id, original.id);
        assert_eq!(rebuilt.metadata.event_type, "aluno.matriculado");
        assert_eq!(
            rebuilt.metadata.occurred_at.timestamp(),
            original.metadata.occurred_at.timestamp(),
        );
        assert_eq!(
            rebuilt
                .metadata
                .correlation_id
                .as_ref()
                .map(CorrelationId::as_str),
            Some("fluxo-1")
        );
        assert_eq!(
            rebuilt
                .metadata
                .causation_id
                .as_ref()
                .map(CausationId::as_str),
            Some("evt-0")
        );
        assert_eq!(rebuilt.metadata.header("tenant"), Some("cmf"));
    }

    #[test]
    fn envelope_sem_metadata_ganha_fallbacks() {
        let properties = BasicProperties::default();
        let rebuilt = envelope_from_properties(&properties, envelope().event, "aluno.matriculado");

        assert_eq!(rebuilt.metadata.event_type, "aluno.matriculado");
        assert!(rebuilt.metadata.correlation_id.is_none());
        assert!(rebuilt.metadata.causation_id.is_none());
        assert!(rebuilt.metadata.headers.is_empty());
    }

    #[test]
    fn redelivery_count_conta_header_x_death() {
        let mut entry = FieldTable::default();
        entry.insert("count".into(), AMQPValue::LongLongInt(2));
        entry.insert("reason".into(), AMQPValue::LongString("rejected".into()));

        let mut headers = FieldTable::default();
        headers.insert(
            "x-death".into(),
            AMQPValue::FieldArray(vec![AMQPValue::FieldTable(entry)].into()),
        );

        let properties = BasicProperties::default().with_headers(headers);

        assert_eq!(redelivery_count(&properties), 2);
        assert_eq!(redelivery_count(&BasicProperties::default()), 0);
    }
}
