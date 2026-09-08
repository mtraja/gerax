//! Eventos: identificação e representação persistida.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::metadata::EventMetadata;
use crate::version::Version;

/// Identificador único de um evento persistido.
///
/// Um *newtype* sobre [`Uuid`] para que a identidade de eventos não
/// seja exposta diretamente como `Uuid` em toda a API pública.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(Uuid);

impl EventId {
    /// Gera um novo identificador aleatório (UUID v4).
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Cria a partir de um [`Uuid`] existente.
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Expõe o [`Uuid`] subjacente.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Evento pronto para persistência.
///
/// Representa a forma *persistida* de um Domain Event: contém a
/// identidade do evento, a identidade e o tipo do Aggregate, o tipo do
/// evento, a versão na stream, o payload serializado, a metadata e o
/// instante em que o evento ocorreu.
///
/// O Domain Event em si (ex.: `StudentCreated`) não conhece estes
/// dados de armazenamento; eles pertencem ao [`StoredEvent`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredEvent {
    /// Identificador único do evento.
    pub event_id: EventId,
    /// Identificador do Aggregate dono da stream.
    pub aggregate_id: String,
    /// Tipo do Aggregate dono da stream.
    pub aggregate_type: String,
    /// Tipo do Domain Event (ex.: `"StudentCreated"`).
    pub event_type: String,
    /// Versão do Aggregate na stream após este evento.
    pub version: Version,
    /// Payload do evento serializado.
    pub payload: Value,
    /// Metadata extensível do evento.
    pub metadata: EventMetadata,
    /// Instante em que o evento ocorreu.
    pub occurred_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_event_carrega_dados_completos() {
        let occurred_at = Utc::now();
        let event = StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "student-1".into(),
            aggregate_type: "Student".into(),
            event_type: "StudentCreated".into(),
            version: Version::new(1),
            payload: serde_json::json!({ "name": "Ana" }),
            metadata: EventMetadata::new(),
            occurred_at,
        };

        assert_eq!(event.aggregate_type, "Student");
        assert_eq!(event.event_type, "StudentCreated");
        assert_eq!(event.version, Version::new(1));
        assert_eq!(event.payload, serde_json::json!({ "name": "Ana" }));
        assert_eq!(event.occurred_at, occurred_at);
    }

    #[test]
    fn stored_event_serializa_e_desserializa() {
        let event = StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "student-1".into(),
            aggregate_type: "Student".into(),
            event_type: "StudentRenamed".into(),
            version: Version::new(2),
            payload: serde_json::json!({ "name": "Bia" }),
            metadata: EventMetadata::new(),
            occurred_at: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        let back: StoredEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn new_gera_identificadores_unicos() {
        assert_ne!(EventId::new(), EventId::new());
    }

    #[test]
    fn from_uuid_e_as_uuid_preservam_o_valor() {
        let uuid = Uuid::new_v4();
        let id = EventId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn tipo_e_igualdade_e_hash() {
        let uuid = Uuid::new_v4();
        let a = EventId::from_uuid(uuid);
        let b = EventId::from_uuid(uuid);
        assert_eq!(a, b);
        assert_eq!(std::collections::HashSet::from([a]).len(), 1);
    }

    #[test]
    fn serializa_e_desserializa_json() {
        let id = EventId::new();
        let json = serde_json::to_string(&id).unwrap();
        let back: EventId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }
}
