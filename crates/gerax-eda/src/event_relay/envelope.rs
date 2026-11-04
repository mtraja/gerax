//! Envelope de integração do Event Relay.
//!
//! Este é um **contrato de transporte** entre contextos: ele preserva a
//! identidade original do evento persistido e os dados necessários para
//! publicação, sem expor tipos do KurrentDB.
//!
//! > Não confundir com o marcador [`crate::IntegrationEvent`], que
//! > indica que um tipo de domínio é um contrato público. O envelope
//! > abaixo é a **forma em trânsito** desse contrato.

use chrono::{DateTime, Utc};
use gerax_es::{EventId, EventMetadata, StoredEvent, Version};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Envelope de um evento de integração.
///
/// ```text
/// IntegrationEvent
///  ├── id               EventId original (nunca regenerado)
///  ├── event_type       contrato lógico estável
///  ├── aggregate_*      origem do evento
///  ├── stream_revision  revisão original na stream (0-based)
///  ├── occurred_at      instante original
///  ├── correlation_id   fluxo de processamento
///  ├── causation_id     evento causador
///  ├── payload          contrato serializado
///  └── metadata         metadados adicionais (gerax-es)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegrationEvent {
    /// Identificador original do evento.
    pub id: EventId,
    /// Nome lógico estável do evento.
    pub event_type: String,
    /// Identificador do Aggregate de origem.
    pub aggregate_id: String,
    /// Tipo do Aggregate de origem.
    pub aggregate_type: String,
    /// Revisão original do evento na stream (0-based).
    pub stream_revision: u64,
    /// Instante original em que o evento ocorreu.
    pub occurred_at: DateTime<Utc>,
    /// Identificador de correlação do fluxo, se conhecido.
    pub correlation_id: Option<String>,
    /// Identificador do evento causador, se conhecido.
    pub causation_id: Option<String>,
    /// Payload serializado do contrato.
    pub payload: Value,
    /// Metadados adicionais (preservados do evento persistido).
    pub metadata: EventMetadata,
}

impl IntegrationEvent {
    /// Cria um envelope de integração.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: EventId,
        event_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        stream_revision: u64,
        occurred_at: DateTime<Utc>,
        payload: Value,
    ) -> Self {
        Self {
            id,
            event_type: event_type.into(),
            aggregate_id: aggregate_id.into(),
            aggregate_type: aggregate_type.into(),
            stream_revision,
            occurred_at,
            correlation_id: None,
            causation_id: None,
            payload,
            metadata: EventMetadata::new(),
        }
    }

    /// Constrói o envelope a partir de um evento persistido.
    ///
    /// A revisão Gerax (`Version`) é convertida para a revisão original
    /// da stream (`version - 1`), e `correlation_id`/`causation_id` são
    /// extraídos da metadata.
    pub fn from_stored(stored: &StoredEvent) -> Self {
        let metadata = stored.metadata.clone();
        let correlation_id = metadata
            .get("correlation_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let causation_id = metadata
            .get("causation_id")
            .and_then(Value::as_str)
            .map(str::to_owned);

        Self {
            id: stored.event_id.clone(),
            event_type: stored.event_type.clone(),
            aggregate_id: stored.aggregate_id.clone(),
            aggregate_type: stored.aggregate_type.clone(),
            stream_revision: stored.version.value().saturating_sub(1),
            occurred_at: stored.occurred_at,
            correlation_id,
            causation_id,
            payload: stored.payload.clone(),
            metadata,
        }
    }

    /// Reconstrói o evento persistido equivalente.
    ///
    /// `stream_revision + 1` produz a versão Gerax. Correlação e
    /// causação são reinseridas na metadata quando ausentes.
    pub fn to_stored_event(&self) -> StoredEvent {
        let mut metadata = self.metadata.clone();
        if let Some(correlation_id) = &self.correlation_id {
            metadata.insert("correlation_id", correlation_id.clone());
        }
        if let Some(causation_id) = &self.causation_id {
            metadata.insert("causation_id", causation_id.clone());
        }

        StoredEvent {
            event_id: self.id.clone(),
            aggregate_id: self.aggregate_id.clone(),
            aggregate_type: self.aggregate_type.clone(),
            event_type: self.event_type.clone(),
            version: Version::new(self.stream_revision.saturating_add(1)),
            payload: self.payload.clone(),
            metadata,
            occurred_at: self.occurred_at,
        }
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

    /// Define a metadata adicional (builder).
    pub fn with_metadata(mut self, metadata: EventMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Insere uma entrada na metadata adicional (builder).
    pub fn with_metadata_entry(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Valida os campos essenciais do envelope.
    pub fn validate(&self) -> Result<(), String> {
        if self.event_type.trim().is_empty() {
            return Err("integration event_type cannot be empty".into());
        }
        if self.aggregate_id.trim().is_empty() {
            return Err("integration aggregate_id cannot be empty".into());
        }
        if self.aggregate_type.trim().is_empty() {
            return Err("integration aggregate_type cannot be empty".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Utc;

    use gerax_es::EventMetadata;

    fn stored() -> StoredEvent {
        let mut metadata = EventMetadata::new();
        metadata.insert("correlation_id", "fluxo-1");
        metadata.insert("causation_id", "evt-0");
        metadata.insert("tenant", "cmf");

        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "aluno-1".into(),
            aggregate_type: "Aluno".into(),
            event_type: "AlunoMatriculado".into(),
            version: Version::new(7),
            payload: serde_json::json!({ "aluno_id": "aluno-1" }),
            metadata,
            occurred_at: Utc::now(),
        }
    }

    #[test]
    fn from_stored_preserva_identidade_e_revisao() {
        let stored = stored();
        let integration = IntegrationEvent::from_stored(&stored);

        assert_eq!(integration.id, stored.event_id);
        assert_eq!(integration.event_type, "AlunoMatriculado");
        assert_eq!(integration.aggregate_id, "aluno-1");
        assert_eq!(integration.aggregate_type, "Aluno");
        assert_eq!(integration.stream_revision, 6);
        assert_eq!(integration.occurred_at, stored.occurred_at);
        assert_eq!(integration.correlation_id.as_deref(), Some("fluxo-1"));
        assert_eq!(integration.causation_id.as_deref(), Some("evt-0"));
        assert_eq!(integration.payload, stored.payload);
    }

    #[test]
    fn roundtrip_para_stored_preserva_versao_e_metadata() {
        let stored = stored();
        let integration = IntegrationEvent::from_stored(&stored);
        let back = integration.to_stored_event();

        assert_eq!(back.event_id, stored.event_id);
        assert_eq!(back.version, stored.version);
        assert_eq!(back.aggregate_id, stored.aggregate_id);
        assert_eq!(back.aggregate_type, stored.aggregate_type);
        assert_eq!(back.event_type, stored.event_type);
        assert_eq!(back.payload, stored.payload);
        assert_eq!(back.metadata, stored.metadata);
    }

    #[test]
    fn builders_definem_rastreabilidade_e_metadata() {
        let event = IntegrationEvent::new(
            EventId::new(),
            "AlunoMatriculado",
            "aluno-1",
            "Aluno",
            0,
            Utc::now(),
            serde_json::json!({}),
        )
        .with_correlation_id("fluxo-2")
        .with_causation_id("evt-1")
        .with_metadata_entry("tenant", "cmf");

        assert_eq!(event.correlation_id.as_deref(), Some("fluxo-2"));
        assert_eq!(event.causation_id.as_deref(), Some("evt-1"));
        assert_eq!(event.metadata.get("tenant"), Some(&Value::from("cmf")));
        assert!(event.validate().is_ok());
    }

    #[test]
    fn validacao_rejeita_campos_vazios() {
        let mut event = IntegrationEvent::new(
            EventId::new(),
            "",
            "aluno-1",
            "Aluno",
            0,
            Utc::now(),
            serde_json::json!({}),
        );
        assert!(event.validate().is_err());

        event.event_type = "AlunoMatriculado".into();
        event.aggregate_id = "".into();
        assert!(event.validate().is_err());
    }

    #[test]
    fn serializa_e_desserializa_json() {
        let event = IntegrationEvent::from_stored(&stored());
        let json = serde_json::to_string(&event).unwrap();
        let back: IntegrationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }
}
