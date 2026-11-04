//! Helpers dos testes de integração do `gerax-kurrentdb`.
//!
//! Os testes exigem um KurrentDB real (e, no relay, também um NATS com
//! JetStream). Sem `KURRENTDB_URL` — e `NATS_URL` no relay — os testes
//! são ignorados com a mensagem `skipped`, seguindo o padrão dos demais
//! adapters do Gerax.

#![allow(dead_code)]

use std::time::Duration;

use chrono::Utc;
use gerax_es::{EventId, EventMetadata, StoredEvent, Version};

/// URL do KurrentDB lida de `KURRENTDB_URL`.
pub fn kurrentdb_url() -> Option<String> {
    std::env::var("KURRENTDB_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Interrompe o teste quando o KurrentDB não está configurado.
pub fn gate_kurrentdb() -> Option<String> {
    let Some(url) = kurrentdb_url() else {
        eprintln!("skipped: set KURRENTDB_URL to run integration tests");
        return None;
    };
    Some(url)
}

/// URL do NATS lida de `NATS_URL`.
pub fn nats_url() -> Option<String> {
    std::env::var("NATS_URL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Interrompe o teste quando KurrentDB ou NATS não estão configurados.
pub fn gate_relay() -> Option<(String, String)> {
    let Some(kurrentdb) = kurrentdb_url() else {
        eprintln!("skipped: set KURRENTDB_URL and NATS_URL to run relay tests");
        return None;
    };
    let Some(nats) = nats_url() else {
        eprintln!("skipped: set KURRENTDB_URL and NATS_URL to run relay tests");
        return None;
    };
    Some((kurrentdb, nats))
}

/// Sufixo único por execução (streams e grupos não colidem em paralelo).
pub fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

/// Timeout das operações de entrega dos testes.
pub const DELIVERY_TIMEOUT: Duration = Duration::from_secs(15);

/// Cria um evento persistido de teste.
pub fn stored_event(
    aggregate_type: &str,
    aggregate_id: &str,
    version: u64,
    event_type: &str,
    payload: serde_json::Value,
) -> StoredEvent {
    let mut metadata = EventMetadata::new();
    metadata.insert("correlation_id", "fluxo-teste");
    metadata.insert("tenant", "cmf");

    StoredEvent {
        event_id: EventId::new(),
        aggregate_id: aggregate_id.to_string(),
        aggregate_type: aggregate_type.to_string(),
        event_type: event_type.to_string(),
        version: Version::new(version),
        payload,
        metadata,
        occurred_at: Utc::now(),
    }
}
