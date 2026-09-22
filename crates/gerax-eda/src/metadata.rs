//! Metadata de eventos publicados.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Identificador de correlação de uma fluxo de processamento.
///
/// Agrupa eventos e comandos da mesma unidade de trabalho (ex.:
/// uma requisição, um caso de uso). *String-backed* para aceitar
/// UUIDs, ids de request ou qualquer outro rastreador.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Cria uma correlação a partir de qualquer string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Referência à string subjacente.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CorrelationId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CorrelationId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identificador do evento que **causou** este evento.
///
/// Tipicamente o [`EventId`](gerax_es::EventId) do evento anterior na
/// cadeia de processamento. *String-backed* por flexibilidade.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CausationId(String);

impl CausationId {
    /// Cria uma causação a partir de qualquer string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Referência à string subjacente.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for CausationId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CausationId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for CausationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Metadata transportada pelo [`EventEnvelope`](crate::EventEnvelope).
///
/// Campos fixos para identidade e rastreio + `headers` genérico para
/// extensão. A metadata **não** é acoplada a nenhum transporte (HTTP,
/// Kafka, AMQP etc.): adapters externos traduzem estes campos para os
/// headers do respectivo protocolo.
///
/// ```text
/// Event
///  ├── event_type        nome lógico estável
///  ├── occurred_at       instante de ocorrência
///  ├── correlation_id    fluxo de processamento
///  ├── causation_id      evento que originou este
///  └── headers           extensível (string → string)
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Nome lógico do evento ( [`Event::event_type()`](crate::Event::event_type) ).
    pub event_type: String,
    /// Instante em que o evento ocorreu.
    pub occurred_at: DateTime<Utc>,
    /// Identificador de correlação do fluxo, se conhecido.
    pub correlation_id: Option<CorrelationId>,
    /// Identificador do evento causador, se conhecido.
    pub causation_id: Option<CausationId>,
    /// Cabeçalhos livres para extensão.
    pub headers: HashMap<String, String>,
}

impl EventMetadata {
    /// Cria metadata para um evento recém-ocorrido (`occurred_at = agora`).
    pub fn new(event_type: impl Into<String>) -> Self {
        Self {
            event_type: event_type.into(),
            occurred_at: Utc::now(),
            correlation_id: None,
            causation_id: None,
            headers: HashMap::new(),
        }
    }

    /// Define a correlação (aceita `&str`, `String` ou [`CorrelationId`]).
    pub fn with_correlation_id(mut self, id: impl Into<CorrelationId>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Define a causação (aceita `&str`, `String` ou [`CausationId`]).
    pub fn with_causation_id(mut self, id: impl Into<CausationId>) -> Self {
        self.causation_id = Some(id.into());
        self
    }

    /// Insere (ou substitui) um header.
    pub fn insert_header(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String> {
        self.headers.insert(key.into(), value.into())
    }

    /// Retorna um header, se existir.
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nova_metadata_carrega_tipo_e_timestamp() {
        let before = Utc::now();
        let metadata = EventMetadata::new("AlunoMatriculado");
        let after = Utc::now();

        assert_eq!(metadata.event_type, "AlunoMatriculado");
        assert!(metadata.occurred_at >= before);
        assert!(metadata.occurred_at <= after);
        assert!(metadata.correlation_id.is_none());
        assert!(metadata.causation_id.is_none());
        assert!(metadata.headers.is_empty());
    }

    #[test]
    fn correlacao_e_causacao_sao_definiveis() {
        let metadata = EventMetadata::new("AlunoMatriculado")
            .with_correlation_id("fluxo-42")
            .with_causation_id("evt-anterior");

        assert_eq!(
            metadata.correlation_id.as_ref().map(CorrelationId::as_str),
            Some("fluxo-42")
        );
        assert_eq!(
            metadata.causation_id.as_ref().map(CausationId::as_str),
            Some("evt-anterior")
        );
    }

    #[test]
    fn headers_sao_inseridos_e_lidos() {
        let mut metadata = EventMetadata::new("AlunoMatriculado");
        assert_eq!(metadata.insert_header("tenant", "cmf"), None);
        assert_eq!(metadata.header("tenant"), Some("cmf"));

        assert_eq!(
            metadata.insert_header("tenant", "outra"),
            Some("cmf".into())
        );
        assert_eq!(metadata.header("tenant"), Some("outra"));
        assert_eq!(metadata.header("inexistente"), None);
    }

    #[test]
    fn serializa_e_desserializa_json() {
        let metadata = EventMetadata::new("AlunoMatriculado")
            .with_correlation_id("fluxo-1")
            .with_causation_id("evt-0");
        let mut metadata = metadata;
        metadata.insert_header("user_id", "u-1");

        let json = serde_json::to_string(&metadata).unwrap();
        let back: EventMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back, metadata);
    }

    #[test]
    fn ids_aceitam_str_e_string() {
        let a: CorrelationId = "x".into();
        let b: CorrelationId = String::from("y").into();
        assert_eq!(a.to_string(), "x");
        assert_eq!(b.as_str(), "y");

        let c: CausationId = "z".into();
        let d: CausationId = String::from("w").into();
        assert_eq!(c.to_string(), "z");
        assert_eq!(d.as_str(), "w");
    }
}
