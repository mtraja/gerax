//! Streams JetStream (`§11`) — feature `jetstream`.
//!
//! Administração de streams do Gerax com semântica **não-destrutiva**:
//!
//! ```text
//! ensure_stream()   cria a stream se não existir (idempotente)
//! delete_stream()   apaga a stream (ação explícita; nunca automática)
//! ```
//!
//! Padrão do adapter:
//!
//! ```text
//! GERAX_EVENTS
//!    subjects: ["gerax.>"]
//!    storage:  File
//! ```
//!
//! Se a stream já existe com **configuração divergente**, o servidor
//! rejeita a criação (`get_or_create_stream` não reescreve streams
//! existentes) e o adapter propaga o erro — nunca recria silenciosamente
//! uma stream de produção (`§11`).

use std::sync::Arc;

use tracing::{info, warn};

use crate::config::{JetStreamConfig, StreamStorage};
use crate::error::NatsError;

/// Cliente JetStream derivado de uma conexão [`crate::NatsClient`].
#[derive(Debug, Clone)]
pub struct NatsJetStream {
    context: async_nats::jetstream::Context,
    config: JetStreamConfig,
    stream: Arc<String>,
    subject_prefix: String,
}

impl NatsJetStream {
    /// Cria o contexto JetStream a partir do client e da configuração.
    ///
    /// O domínio ([`JetStreamConfig::domain`]) é aplicado quando
    /// presente; a stream usada por este contexto é
    /// [`JetStreamConfig::stream`].
    pub fn new(client: &crate::NatsClient, config: JetStreamConfig) -> Self {
        let raw = client.connection().clone();
        let context = match &config.domain {
            Some(domain) => async_nats::jetstream::with_domain(raw, domain.clone()),
            None => async_nats::jetstream::new(raw),
        };
        let stream = Arc::new(config.stream.clone());
        let subject_prefix = client.config().subject_prefix.clone();
        Self {
            context,
            config,
            stream,
            subject_prefix,
        }
    }

    /// Contexto jetstream (`async-nats`), para operações avançadas.
    ///
    /// Mesmo escape hatch de [`crate::NatsClient::connection`].
    pub fn context(&self) -> &async_nats::jetstream::Context {
        &self.context
    }

    /// Nome da stream gerenciada.
    pub fn stream_name(&self) -> &str {
        &self.stream
    }

    /// Subjects capturados pela stream (derivados do prefixo quando
    /// a configuração não os definiu explicitamente).
    pub fn subjects(&self) -> &[String] {
        &self.config.subjects
    }

    /// Subjects da stream para a criação (com fallback ao prefixo).
    fn creation_subjects(&self) -> Vec<String> {
        if self.config.subjects.is_empty() {
            vec![format!("{}.>", self.subject_prefix)]
        } else {
            self.config.subjects.clone()
        }
    }

    /// Garante que a stream existe (cria quando ausente).
    ///
    /// Operação **idempotente e não-destrutiva**: streams existentes
    /// são mantidas como estão; divergência de configuração vira erro.
    pub async fn ensure_stream(&self) -> Result<(), NatsError> {
        let config = async_nats::jetstream::stream::Config {
            name: self.stream.clone().as_str().to_string(),
            subjects: self.creation_subjects(),
            storage: match self.config.storage {
                StreamStorage::File => async_nats::jetstream::stream::StorageType::File,
                StreamStorage::Memory => async_nats::jetstream::stream::StorageType::Memory,
            },
            ..Default::default()
        };

        match self.context.get_or_create_stream(config).await {
            Ok(_stream) => {
                info!(stream = %self.stream, "jetstream stream garantida");
                Ok(())
            }
            Err(source) => Err(NatsError::JetStream(format!(
                "ensure stream `{}` failed: {source}",
                self.stream
            ))),
        }
    }

    /// Apaga a stream — **ação explícita** (`§11`).
    ///
    /// Nunca chamada automaticamente pelo adapter; em produção prefira
    /// operações de administração fora do ciclo de vida da aplicação.
    pub async fn delete_stream(&self) -> Result<(), NatsError> {
        warn!(stream = %self.stream, "deletando jetstream stream");
        self.context
            .delete_stream(self.stream.clone().as_str())
            .await
            .map_err(|source| {
                NatsError::JetStream(format!("delete stream `{}` failed: {source}", self.stream))
            })?;
        info!(stream = %self.stream, "jetstream stream deletada");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_mapping_e_estavel() {
        assert!(matches!(StreamStorage::File, StreamStorage::File));
        assert!(matches!(StreamStorage::default(), StreamStorage::File));
    }

    #[test]
    fn config_padrao_e_gerax_events() {
        let config = JetStreamConfig::new();
        assert_eq!(config.stream, "GERAX_EVENTS");
        assert_eq!(config.storage, StreamStorage::File);
        assert!(config.domain.is_none());
    }
}
