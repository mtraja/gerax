//! Envelope de eventos publicados.

use gerax_es::{EventId, StoredEvent};

use crate::event::Event;
use crate::metadata::EventMetadata;

/// Envelope que transporta um evento pelo bus, **independente do
/// payload**.
///
/// ```text
/// EventEnvelope
///  ├── id              EventId (uuid; preservado se persistido)
///  ├── event           payload tipado (E)
///  └── metadata
///       ├── event_type
///       ├── occurred_at
///       ├── correlation_id
///       ├── causation_id
///       └── headers
/// ```
///
/// O envelope é a unidade entregue aos
/// [`EventHandler`](crate::EventHandler)s. O handler conhece o evento,
/// mas não o mecanismo de transporte.
///
/// # Identidade
///
/// [`EventEnvelope::new`] gera um novo [`EventId`]. Quando o evento já
/// foi persistido no `gerax-es`, use [`EventEnvelope::from_stored`]:
/// o identificador, o timestamp e a metadata do
/// [`StoredEvent`] são **preservados** — nenhum UUID novo é gerado.
#[derive(Debug, Clone, PartialEq)]
pub struct EventEnvelope<E> {
    /// Identificador único desta entrega do evento.
    pub id: EventId,
    /// Payload tipado do evento.
    pub event: E,
    /// Metadata de rastreio e extensão.
    pub metadata: EventMetadata,
}

impl<E> EventEnvelope<E>
where
    E: Event,
{
    /// Cria um envelope para um evento recém-publicado.
    ///
    /// - `id` — novo [`EventId`] (UUID v4);
    /// - `metadata.event_type` — derivado de [`Event::event_type`];
    /// - `metadata.occurred_at` — agora.
    pub fn new(event: E) -> Self {
        let metadata = EventMetadata::new(event.event_type());
        Self {
            id: EventId::new(),
            event,
            metadata,
        }
    }

    /// Constrói o envelope a partir de um evento **já persistido**.
    ///
    /// Preserva `event_id` e `occurred_at` do [`StoredEvent`] e
    /// copia correlação/causação quando presentes na metadata
    /// persistida (`gerax-es` guarda metadata como mapa genérico;
    /// chaves string de outros valores viram headers do envelope).
    pub fn from_stored(event: E, stored: &StoredEvent) -> Self {
        let mut metadata = EventMetadata {
            event_type: stored.event_type.clone(),
            occurred_at: stored.occurred_at,
            correlation_id: None,
            causation_id: None,
            headers: Default::default(),
        };

        for (key, value) in stored.metadata.iter() {
            let Some(text) = value.as_str() else {
                continue;
            };
            match key.as_str() {
                "correlation_id" => metadata.correlation_id = Some(text.into()),
                "causation_id" => metadata.causation_id = Some(text.into()),
                _ => {
                    metadata.headers.insert(key.clone(), text.to_string());
                }
            }
        }

        Self {
            id: stored.event_id.clone(),
            event,
            metadata,
        }
    }

    /// Define a correlação do envelope (builder).
    pub fn with_correlation_id(mut self, id: impl Into<crate::metadata::CorrelationId>) -> Self {
        self.metadata.correlation_id = Some(id.into());
        self
    }

    /// Define a causação do envelope (builder).
    pub fn with_causation_id(mut self, id: impl Into<crate::metadata::CausationId>) -> Self {
        self.metadata.causation_id = Some(id.into());
        self
    }

    /// Insere um header no envelope (builder).
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert_header(key, value);
        self
    }

    /// Nome lógico do evento (conveniência sobre `metadata.event_type`).
    pub fn event_type(&self) -> &str {
        &self.metadata.event_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use gerax_es::EventMetadata as StoredMetadata;
    use gerax_es::Version;

    #[derive(Debug, Clone, PartialEq)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    fn aluno() -> AlunoMatriculado {
        AlunoMatriculado {
            aluno_id: "a-1".into(),
        }
    }

    fn stored(event_type: &str, metadata: StoredMetadata) -> StoredEvent {
        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "a-1".into(),
            aggregate_type: "Aluno".into(),
            event_type: event_type.into(),
            version: Version::new(1),
            payload: serde_json::json!({}),
            metadata,
            occurred_at: Utc::now(),
        }
    }

    #[test]
    fn new_gera_id_e_metadata_derivada_do_evento() {
        let before = Utc::now();
        let envelope = EventEnvelope::new(aluno());
        let after = Utc::now();

        assert_eq!(envelope.metadata.event_type, "aluno.matriculado");
        assert_eq!(envelope.event_type(), "aluno.matriculado");
        assert!(envelope.metadata.occurred_at >= before);
        assert!(envelope.metadata.occurred_at <= after);
        assert!(envelope.metadata.correlation_id.is_none());
    }

    #[test]
    fn novos_envelopes_tem_ids_distintos() {
        assert_ne!(
            EventEnvelope::new(aluno()).id,
            EventEnvelope::new(aluno()).id
        );
    }

    #[test]
    fn builders_definem_correlacao_causacao_e_header() {
        let envelope = EventEnvelope::new(aluno())
            .with_correlation_id("fluxo-1")
            .with_causation_id("evt-0")
            .with_header("tenant", "cmf");

        assert_eq!(
            envelope
                .metadata
                .correlation_id
                .as_ref()
                .map(|c| c.as_str()),
            Some("fluxo-1")
        );
        assert_eq!(
            envelope.metadata.causation_id.as_ref().map(|c| c.as_str()),
            Some("evt-0")
        );
        assert_eq!(envelope.metadata.header("tenant"), Some("cmf"));
    }

    #[test]
    fn from_stored_preserva_id_e_timestamp_sem_gerar_novo_uuid() {
        let persistido = stored("aluno.matriculado", StoredMetadata::new());
        let envelope = EventEnvelope::from_stored(aluno(), &persistido);

        assert_eq!(envelope.id, persistido.event_id);
        assert_eq!(envelope.metadata.occurred_at, persistido.occurred_at);
        assert_eq!(envelope.metadata.event_type, "aluno.matriculado");
    }

    #[test]
    fn from_stored_mapeia_correlacao_causacao_e_headers() {
        let mut persisted = StoredMetadata::new();
        persisted.insert("correlation_id", "fluxo-9");
        persisted.insert("causation_id", "evt-8");
        persisted.insert("user_id", "u-1");
        persisted.insert("contador", 7); // não-string: ignorado

        let envelope = EventEnvelope::from_stored(aluno(), &stored("aluno.matriculado", persisted));

        assert_eq!(
            envelope
                .metadata
                .correlation_id
                .as_ref()
                .map(|c| c.as_str()),
            Some("fluxo-9")
        );
        assert_eq!(
            envelope.metadata.causation_id.as_ref().map(|c| c.as_str()),
            Some("evt-8")
        );
        assert_eq!(envelope.metadata.header("user_id"), Some("u-1"));
        assert_eq!(envelope.metadata.header("contador"), None);
    }
}
