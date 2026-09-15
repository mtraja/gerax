//! Formatação de eventos Server-Sent Events (SSE).

/// Evento SSE individual.
///
/// Cada evento carrega um `id` globalmente único dentro da sessão e um
/// `data`. A linha `data` do evento vazio é usada para "prime" a
/// reconexão do cliente.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// Identificador do evento (cursor para `Last-Event-ID`).
    pub id: String,
    /// Conteúdo do evento (JSON-RPC serializado ou vazio para prime).
    pub data: String,
}

impl SseEvent {
    /// Serializa o evento no formato `text/event-stream`.
    pub fn to_body(&self) -> String {
        format!("id: {}\ndata: {}\n\n", self.id, self.data)
    }
}

/// Evento de "prime": `id` do segundo evento seguinte + `data` vazio.
///
/// Enviado imediatamente quando uma stream SSE é aberta, para o cliente
/// saber o último id recebido e reconectar via `Last-Event-ID`.
pub fn prime_event(session: &crate::SessionId, next_sequence: u64) -> SseEvent {
    SseEvent {
        id: format!("{session}:{next_sequence}"),
        data: String::new(),
    }
}

/// Sequência de eventos para mensagens JSON-RPC.
///
/// Cada [`crate::sse::SseEvent`] recebe um id sequencial `{sessão}:{n}`.
pub fn event_sequence(
    session: &crate::SessionId,
    messages: &[serde_json::Value],
    start_sequence: u64,
) -> Vec<SseEvent> {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| SseEvent {
            id: format!("{session}:{}", start_sequence + index as u64),
            data: message.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prime_event_has_empty_data_and_sequence_id() {
        let event = prime_event(&"abc".to_owned(), 1);

        assert_eq!(event.id, "abc:1");
        assert_eq!(event.data, "");
    }

    #[test]
    fn event_to_body_follows_sse_framing() {
        let event = SseEvent {
            id: "s:1".to_owned(),
            data: r#"{"jsonrpc":"2.0"}"#.to_owned(),
        };

        assert_eq!(event.to_body(), "id: s:1\ndata: {\"jsonrpc\":\"2.0\"}\n\n");
    }

    #[test]
    fn event_sequence_increments_ids() {
        let session = "sessao".to_owned();
        let messages = vec![json!({"a": 1}), json!({"b": 2})];

        let events = event_sequence(&session, &messages, 3);

        assert_eq!(events[0].id, "sessao:3");
        assert_eq!(events[1].id, "sessao:4");
        assert_eq!(events[1].data, r#"{"b":2}"#);
    }
}
