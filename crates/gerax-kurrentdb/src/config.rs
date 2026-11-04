//! Configuração de conexão e de subscriptions do KurrentDB.
//!
//! A configuração pode ser construída por código ([`KurrentDbConfig`]) ou
//! carregada de TOML/JSON. O formato de arquivo segue o exemplo:
//!
//! ```toml
//! [kurrentdb]
//! connection_string = "kurrentdb://admin:changeit@localhost:2113?tls=false"
//!
//! [kurrentdb.subscription]
//! stream = "$all"
//! group = "gerax-event-relay"
//! buffer_size = 100
//! max_retry_count = 5
//! ```
//!
//! Credenciais não devem ser versionadas. Prefira variáveis de ambiente
//! ou um gerenciador de segredos para montar a connection string.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::KurrentDbError;
use crate::subscription::config::SubscriptionConfig;

/// Connection string padrão para um KurrentDB local sem TLS.
pub const DEFAULT_CONNECTION_STRING: &str = "kurrentdb://localhost:2113?tls=false";

/// Configuração do adapter KurrentDB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KurrentDbConfig {
    /// Connection string do KurrentDB (`kurrentdb://...` ou `esdb://...`).
    pub connection_string: String,

    /// Configuração opcional de uma subscription padrão da aplicação.
    ///
    /// O grupo é uma configuração **administrativa**: a aplicação pode
    /// usá-la para criar/garantir o grupo explicitamente. O consumidor
    /// nunca cria o grupo automaticamente.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription: Option<SubscriptionConfig>,
}

impl KurrentDbConfig {
    /// Cria a configuração a partir de uma connection string.
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            subscription: None,
        }
    }

    /// Adiciona a configuração de subscription (builder).
    pub fn with_subscription(mut self, subscription: SubscriptionConfig) -> Self {
        self.subscription = Some(subscription);
        self
    }

    /// Valida a configuração de conexão.
    pub fn validate(&self) -> Result<(), KurrentDbError> {
        if self.connection_string.trim().is_empty() {
            return Err(KurrentDbError::Configuration(
                "connection_string cannot be empty".into(),
            ));
        }
        if let Some(subscription) = &self.subscription {
            subscription.validate().map_err(|error| {
                KurrentDbError::Configuration(format!("invalid subscription config: {error}"))
            })?;
        }
        Ok(())
    }

    /// Deserializa a configuração de uma string TOML.
    pub fn from_toml_str(input: &str) -> Result<Self, KurrentDbError> {
        toml::from_str(input).map_err(|error| {
            KurrentDbError::Configuration(format!("invalid TOML configuration: {error}"))
        })
    }

    /// Lê a configuração de um arquivo TOML.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self, KurrentDbError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .map_err(|error| KurrentDbError::Io(format!("{}: {error}", path.display())))?;
        Self::from_toml_str(&contents)
    }

    /// Deserializa a configuração de uma string JSON.
    pub fn from_json_str(input: &str) -> Result<Self, KurrentDbError> {
        serde_json::from_str(input).map_err(|error| {
            KurrentDbError::Configuration(format!("invalid JSON configuration: {error}"))
        })
    }
}

impl Default for KurrentDbConfig {
    fn default() -> Self {
        Self::new(DEFAULT_CONNECTION_STRING)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuracao_padrao_aponta_para_localhost() {
        let config = KurrentDbConfig::default();
        assert!(config.connection_string.contains("localhost:2113"));
        assert!(config.subscription.is_none());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn configuracoes_vazias_sao_rejeitadas() {
        let config = KurrentDbConfig::new("   ");
        assert!(matches!(
            config.validate(),
            Err(KurrentDbError::Configuration(_))
        ));
    }

    #[test]
    fn carrega_toml_com_subscription() {
        let config = KurrentDbConfig::from_toml_str(
            r#"
connection_string = "kurrentdb://localhost:2113?tls=false"

[subscription]
stream = "$all"
group = "relay"
buffer_size = 32
max_retry_count = 3
"#,
        )
        .unwrap();

        let subscription = config.subscription.as_ref().expect("subscription");
        assert_eq!(subscription.stream, "$all");
        assert_eq!(subscription.group, "relay");
        assert_eq!(subscription.buffer_size, 32);
        assert_eq!(subscription.max_retry_count, 3);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn carrega_json() {
        let config = KurrentDbConfig::from_json_str(
            r#"{"connection_string":"kurrentdb://localhost:2113?tls=false"}"#,
        )
        .unwrap();
        assert_eq!(config, KurrentDbConfig::default());
    }

    #[test]
    fn toml_invalido_vira_configuration_error() {
        assert!(matches!(
            KurrentDbConfig::from_toml_str("connection_string = ["),
            Err(KurrentDbError::Configuration(_))
        ));
    }
}
