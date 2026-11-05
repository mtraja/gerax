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

use std::env;

/// Modo de execução de projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RunProjections {
    /// Nenhuma projection habilitada.
    #[default]
    None,
    /// Apenas projections do sistema.
    System,
    /// Todas as projections habilitadas.
    All,
}

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

    /// Modo de desenvolvimento: permite conexões sem TLS.
    #[serde(default = "default_dev_mode")]
    pub dev_mode: bool,

    /// Configuração de execução de projections.
    #[serde(default)]
    pub run_projections: RunProjections,

    /// Permite habilitar todas as projections em produção explicitamente.
    #[serde(default)]
    pub allow_all_projections_in_production: bool,
}

impl KurrentDbConfig {
    /// Cria a configuração a partir de uma connection string.
    pub fn new(connection_string: impl Into<String>) -> Self {
        let dev_mode = env::var("GERAX_KURRENTDB_DEV_MODE")
            .or_else(|_| env::var("DEV_MODE"))
            .is_ok_and(|v| matches!(v.as_str().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"));

        let run_projections = env::var("GERAX_KURRENTDB_RUN_PROJECTIONS")
            .ok()
            .and_then(|v| match v.as_str().to_ascii_lowercase().as_str() {
                "none" => Some(RunProjections::None),
                "system" => Some(RunProjections::System),
                "all" => Some(RunProjections::All),
                _ => None,
            })
            .unwrap_or_default();

        let allow_all_projections_in_production = env::var("GERAX_KURRENTDB_ALLOW_ALL_PROJECTIONS")
            .or_else(|_| env::var("ALLOW_ALL_PROJECTIONS_IN_PRODUCTION"))
            .or_else(|_| env::var("KURRENTDB_ALLOW_ALL_PROJECTIONS"))
            .is_ok_and(|v| matches!(v.as_str().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"));

        Self {
            connection_string: connection_string.into(),
            subscription: None,
            dev_mode,
            run_projections,
            allow_all_projections_in_production,
        }
    }

    /// Adiciona a configuração de subscription (builder).
    pub fn with_subscription(mut self, subscription: SubscriptionConfig) -> Self {
        self.subscription = Some(subscription);
        self
    }

    /// Define o modo de desenvolvimento.
    pub fn with_dev_mode(mut self, dev_mode: bool) -> Self {
        self.dev_mode = dev_mode;
        self
    }

    /// Define o modo de execução de projections.
    pub fn with_run_projections(mut self, run_projections: RunProjections) -> Self {
        self.run_projections = run_projections;
        self
    }

    /// Permite habilitar todas as projections em produção.
    pub fn with_allow_all_projections_in_production(mut self, allow: bool) -> Self {
        self.allow_all_projections_in_production = allow;
        self
    }

    /// Valida a configuração de conexão.
    pub fn validate(&self) -> Result<(), KurrentDbError> {
        if self.connection_string.trim().is_empty() {
            return Err(KurrentDbError::Configuration(
                "connection_string cannot be empty".into(),
            ));
        }

        // Validação de projections
        if matches!(self.run_projections, RunProjections::All) && !self.dev_mode && !self.allow_all_projections_in_production {
            return Err(KurrentDbError::Configuration(
                "run_projections=all is not allowed in production; enable dev_mode or set allow_all_projections_in_production=true".into(),
            ));
        }

        let conn = self.connection_string.to_ascii_lowercase();
        if !self.dev_mode {
            // Reject insecure/mem-db settings in non-dev mode
            if conn.contains("tls=false") || conn.contains("tls:") == false && conn.contains("kurrentdb://") && !conn.contains("tls=true") && conn.contains("?") {
                // best-effort check: if tls explicitly false or not specified in some forms, be strict
            }
            if conn.contains("tls=false") || conn.contains("inmem") || conn.contains("memdb") {
                return Err(KurrentDbError::Configuration(
                    "insecure or in-memory connection strings are not allowed in non-dev mode".into(),
                ));
            }
            if !conn.contains("tls=true") {
                // also require tls unless dev_mode true
                return Err(KurrentDbError::Configuration(
                    "connection string must use tls in non-dev mode".into(),
                ));
            }
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

fn default_dev_mode() -> bool {
    true
}

impl Default for KurrentDbConfig {
    fn default() -> Self {
        let mut cfg = KurrentDbConfig::new(DEFAULT_CONNECTION_STRING);
        cfg.dev_mode = true;
        cfg.run_projections = RunProjections::None;
        cfg.allow_all_projections_in_production = false;
        cfg
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
