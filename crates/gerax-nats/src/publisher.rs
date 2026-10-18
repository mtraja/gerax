//! Publicação de eventos (`§10`).
//!
//! [`NatsPublisher`] publica [`NatsEnvelope`]s no subject derivado do
//! `event_type` (`<prefixo>.<event_type>`), com headers NATS de
//! rastreabilidade e dedup:
//!
//! ```text
//! Core NATS   publish ──► subject          + flush (confirmação local)
//! JetStream   publish ──► stream ──► PubAck (confirmação do servidor)
//! ```
//!
//! Para eventos persistentes de EDA, **prefira JetStream**
//! ([`NatsConfig::jetstream`] habilitado): o `PubAck` é a confirmação
//! de que a mensagem foi persistida na stream (`§10`). Em Core NATS o
//! `flush` confirma apenas a saída do buffer do cliente — sem
//! durabilidade.

use std::time::Instant;

#[cfg(feature = "jetstream")]
use tracing::debug;
use tracing::info;

use crate::client::NatsClient;
use crate::envelope::{self, NatsEnvelope};
use crate::error::NatsError;
use crate::subject::SubjectBuilder;

/// Modo de publicação derivado da configuração.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishMode {
    /// Core NATS (transitório; confirmação local via flush).
    Core,
    /// JetStream (persistente; confirmação via PubAck).
    JetStream,
}

/// Publicador de envelopes NATS.
///
/// Seguro para concorrência: o [`async_nats::Client`] interno
/// multiplexa a conexão, e cada publicação JetStream aguarda o seu
/// `PubAck`.
#[derive(Debug, Clone)]
pub struct NatsPublisher {
    client: async_nats::Client,
    subjects: SubjectBuilder,
    request_timeout: std::time::Duration,
    #[cfg(feature = "jetstream")]
    jetstream: Option<async_nats::jetstream::Context>,
    mode: PublishMode,
}

impl NatsPublisher {
    /// Cria o publicador a partir de uma conexão [`NatsClient`].
    ///
    /// Com [`crate::NatsConfig::jetstream`] habilitado (e feature
    /// `jetstream` ativa), cada publicação aguarda o `PubAck` do
    /// servidor (at-least-once com persistência); caso contrário
    /// publica em Core NATS com flush.
    pub fn new(client: &NatsClient) -> Self {
        let subjects = client
            .subjects()
            .expect("prefixo de subject validado no build da config");

        #[cfg(feature = "jetstream")]
        let (jetstream_context, mode) = {
            let context = client.config().jetstream.as_ref().map(|js| {
                let raw = client.connection().clone();
                match &js.domain {
                    Some(domain) => async_nats::jetstream::with_domain(raw, domain.clone()),
                    None => async_nats::jetstream::new(raw),
                }
            });
            let mode = if context.is_some() {
                PublishMode::JetStream
            } else {
                PublishMode::Core
            };
            (context, mode)
        };
        #[cfg(not(feature = "jetstream"))]
        let mode = PublishMode::Core;

        Self {
            client: client.connection().clone(),
            subjects,
            request_timeout: client.request_timeout(),
            #[cfg(feature = "jetstream")]
            jetstream: jetstream_context,
            mode,
        }
    }

    /// Modo de publicação ativo.
    ///
    /// Sem a feature `jetstream`, o modo é sempre [`PublishMode::Core`]
    /// — mesmo que a configuração peça JetStream.
    pub const fn mode(&self) -> PublishMode {
        self.mode
    }

    /// Builder de subjects do publicador.
    pub fn subjects(&self) -> &SubjectBuilder {
        &self.subjects
    }

    /// Publica um envelope de transporte no subject do seu `event_type`.
    ///
    /// - valida o subject contra a política (`§9`);
    /// - serializa o envelope em JSON com headers `Nats-Msg-Id`
    ///   (dedup JetStream) e rastreabilidade;
    /// - JetStream: aguarda [`async_nats::jetstream::PublishAck`];
    /// - Core NATS: `flush` após o publish.
    pub async fn publish_envelope(&self, envelope: &NatsEnvelope) -> Result<(), NatsError> {
        let subject = self.subjects.event(&envelope.event_type)?;
        let headers = envelope::nats_headers(envelope);
        let payload = envelope::encode_envelope(envelope)?;
        let started = Instant::now();

        #[cfg(feature = "jetstream")]
        if let Some(context) = &self.jetstream
            && self.mode == PublishMode::JetStream
        {
            let ack = context
                .publish_with_headers(subject.clone(), headers, payload)
                .await
                .map_err(|source| NatsError::Publish(format!("{source}")))?
                .await
                .map_err(|source| NatsError::Publish(format!("{source}")))?;
            debug!(
                event_id = %envelope.event_id.as_deref().unwrap_or("-"),
                event_type = %envelope.event_type,
                subject = %subject,
                stream = %ack.stream,
                sequence = ack.sequence,
                mode = "jetstream",
                duration_ms = started.elapsed().as_millis() as u64,
                "evento publicado"
            );
            return Ok(());
        }

        self.client
            .publish_with_headers(subject.clone(), headers, payload)
            .await
            .map_err(|source| NatsError::Publish(format!("{source}")))?;
        match tokio::time::timeout(self.request_timeout, self.client.flush()).await {
            Ok(result) => result.map_err(|source| NatsError::Connection(source.to_string()))?,
            Err(_) => return Err(NatsError::Timeout),
        }
        info!(
            event_id = %envelope.event_id.as_deref().unwrap_or("-"),
            event_type = %envelope.event_type,
            subject = %subject,
            mode = "core",
            duration_ms = started.elapsed().as_millis() as u64,
            "evento publicado"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_mode_e_estavel() {
        assert!(matches!(PublishMode::Core, PublishMode::Core));
        assert!(matches!(PublishMode::JetStream, PublishMode::JetStream));
    }

    #[test]
    fn publisher_e_cloneavel() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<NatsPublisher>();
    }
}
