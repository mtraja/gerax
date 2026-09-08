//! Serialização de eventos.

use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::SerializationError;

/// Evento de domínio convertido em formato persistível.
///
/// É a saída de [`EventSerializer::serialize`]: o tipo canônico do
/// evento e o payload serializado. O restante dos campos do
/// [`crate::StoredEvent`] (identidades, versão, metadata, timestamp) é
/// preenchido pelo [`crate::AggregateRepository`].
#[derive(Debug, Clone, PartialEq)]
pub struct SerializedEvent {
    /// Tipo canônico do Domain Event.
    pub event_type: String,
    /// Payload do evento serializado.
    pub payload: Value,
}

/// Conversão entre Domain Events e payload persistível.
///
/// Abstração que permite trocar o formato de persistência (JSON,
/// MessagePack, Bincode etc.) sem alterar o restante da API.
///
/// ```text
/// Domain Event ──serialize──▶ SerializedEvent
/// SerializedEvent ──deserialize──▶ Domain Event
/// ```
pub trait EventSerializer: Send + Sync {
    /// Tipo do Domain Event serializado.
    type Event;

    /// Tipo de erro da serialização.
    type Error;

    /// Serializa um Domain Event em [`SerializedEvent`].
    fn serialize(&self, event: &Self::Event) -> Result<SerializedEvent, Self::Error>;

    /// Desserializa um payload de volta em um Domain Event.
    ///
    /// `event_type` indica qual tipo de evento o payload representa.
    fn deserialize(&self, event_type: &str, payload: &Value) -> Result<Self::Event, Self::Error>;
}

/// Serializador JSON baseado em `serde_json`.
///
/// Requer que o tipo de evento seja um enum com **tag interna** chamada
/// `event_type`:
///
/// ```rust
/// #[derive(serde::Serialize, serde::Deserialize)]
/// #[serde(tag = "event_type")]
/// enum StudentEvent {
///     StudentCreated { name: String },
///     StudentRenamed { name: String },
/// }
/// ```
///
/// A tag interna é usada para derivar o `event_type` na serialização e
/// para validar o tipo na desserialização. Um tipo de evento não
/// reconhecido produz erro explícito [`SerializationError::UnknownEventType`] —
/// nunca é ignorado silenciosamente.
///
/// JSON não é uma limitação arquitetural permanente: novos formatos
/// podem ser adicionados implementando [`EventSerializer`].
pub struct JsonEventSerializer<E> {
    _marker: PhantomData<E>,
}

impl<E> JsonEventSerializer<E> {
    /// Cria um serializador JSON para o tipo `E`.
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<E> Default for JsonEventSerializer<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> EventSerializer for JsonEventSerializer<E>
where
    E: Serialize + DeserializeOwned + Send + Sync,
{
    type Event = E;
    type Error = SerializationError;

    fn serialize(&self, event: &E) -> Result<SerializedEvent, SerializationError> {
        let payload = serde_json::to_value(event)
            .map_err(|source| SerializationError::Serialize { source })?;

        let event_type = payload
            .get("event_type")
            .and_then(Value::as_str)
            .ok_or(SerializationError::MissingEventType)?;

        Ok(SerializedEvent {
            event_type: event_type.to_string(),
            payload,
        })
    }

    fn deserialize(&self, event_type: &str, payload: &Value) -> Result<E, SerializationError> {
        let declared = payload.get("event_type").and_then(Value::as_str);
        if declared != Some(event_type) {
            return Err(SerializationError::UnknownEventType(event_type.to_string()));
        }

        match serde_json::from_value::<E>(payload.clone()) {
            Ok(event) => Ok(event),
            Err(source) if source.to_string().contains("unknown variant") => {
                Err(SerializationError::UnknownEventType(event_type.to_string()))
            }
            Err(source) => Err(SerializationError::InvalidPayload {
                event_type: event_type.to_string(),
                source,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
    #[serde(tag = "event_type")]
    enum StudentEvent {
        StudentCreated { name: String },
        StudentRenamed { name: String },
    }

    #[test]
    fn serialize_produz_event_type_e_payload() {
        let serializer = JsonEventSerializer::<StudentEvent>::new();

        let serialized = serializer
            .serialize(&StudentEvent::StudentCreated { name: "Ana".into() })
            .unwrap();

        assert_eq!(serialized.event_type, "StudentCreated");
        assert_eq!(serialized.payload["name"], "Ana");
        assert_eq!(serialized.payload["event_type"], "StudentCreated");
    }

    #[test]
    fn deserialize_roundtrip_preserva_o_evento() {
        let serializer = JsonEventSerializer::<StudentEvent>::new();
        let original = StudentEvent::StudentRenamed { name: "Bia".into() };

        let serialized = serializer.serialize(&original).unwrap();
        let back = serializer
            .deserialize(&serialized.event_type, &serialized.payload)
            .unwrap();

        assert_eq!(back, original);
    }

    #[test]
    fn deserialize_redorna_para_outro_variante_do_enum() {
        let serializer = JsonEventSerializer::<StudentEvent>::new();
        let criado = StudentEvent::StudentCreated { name: "Ana".into() };

        let serialized = serializer.serialize(&criado).unwrap();
        let back = serializer
            .deserialize(&serialized.event_type, &serialized.payload)
            .unwrap();

        assert_eq!(back, StudentEvent::StudentCreated { name: "Ana".into() });
    }

    #[test]
    fn deserialize_payload_invalido_produz_erro_explicito() {
        let serializer = JsonEventSerializer::<StudentEvent>::new();
        let payload = serde_json::json!({ "event_type": "StudentCreated" });

        let err = serializer
            .deserialize("StudentCreated", &payload)
            .unwrap_err();

        assert!(matches!(err, SerializationError::InvalidPayload { .. }));
    }

    #[test]
    fn deserialize_tipo_desconhecido_produz_erro_explicito() {
        let serializer = JsonEventSerializer::<StudentEvent>::new();
        let payload = serde_json::json!({ "event_type": "StudentGraduated", "name": "Ana" });

        let err = serializer
            .deserialize("StudentGraduated", &payload)
            .unwrap_err();

        assert!(matches!(err, SerializationError::UnknownEventType(_)));
        assert_eq!(err.to_string(), "unknown event type `StudentGraduated`");
    }

    #[test]
    fn desserializar_lida_de_um_tipo_diferente_rejeita() {
        let serializer = JsonEventSerializer::<StudentEvent>::new();
        let criado = StudentEvent::StudentCreated { name: "Ana".into() };
        let serialized = serializer.serialize(&criado).unwrap();

        let err = serializer
            .deserialize("StudentRenamed", &serialized.payload)
            .unwrap_err();

        assert!(matches!(err, SerializationError::UnknownEventType(_)));
    }
}
