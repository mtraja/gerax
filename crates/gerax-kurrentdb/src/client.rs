//! Cliente KurrentDB compartilhado.
//!
//! O mesmo cliente pode ser reutilizado pelo
//! [`KurrentDbEventStore`](crate::event_store::KurrentDbEventStore) e por
//! [`KurrentDbSubscription`](crate::subscription::KurrentDbSubscription).
//! Cada funcionalidade é independente: criar uma não exige a outra.

use kurrentdb::{Client, ClientSettings};
use tracing::{debug, info};

use crate::config::KurrentDbConfig;
use crate::error::KurrentDbError;

/// Cliente KurrentDB compartilhado pelo Event Store e pelas subscriptions.
///
/// A conexão é multiplexada internamente pelo cliente oficial e pode ser
/// clonada sem abrir novas conexões.
#[derive(Clone)]
pub struct KurrentDbClient {
    client: Client,
}

impl KurrentDbClient {
    /// Conecta a partir de uma connection string.
    ///
    /// Exemplos:
    ///
    /// ```text
    /// kurrentdb://localhost:2113?tls=false
    /// kurrentdb://admin:changeit@localhost:2113?tls=false
    /// kurrentdb://admin:changeit@node-1:2113,node-2:2113
    /// ```
    pub async fn connect(connection_string: &str) -> Result<Self, KurrentDbError> {
        if connection_string.trim().is_empty() {
            return Err(KurrentDbError::Configuration(
                "connection_string cannot be empty".into(),
            ));
        }

        let settings = connection_string
            .parse::<ClientSettings>()
            .map_err(|error| KurrentDbError::InvalidConnectionString(error.to_string()))?;

        Self::from_settings(settings)
    }

    /// Conecta a partir da configuração do adapter.
    pub async fn from_config(config: &KurrentDbConfig) -> Result<Self, KurrentDbError> {
        config.validate()?;
        Self::connect(&config.connection_string).await
    }

    /// Cria o cliente a partir de [`ClientSettings`] já validados.
    pub fn from_settings(settings: ClientSettings) -> Result<Self, KurrentDbError> {
        let client = Client::new(settings)
            .map_err(|error| KurrentDbError::ClientInitialization(error.to_string()))?;
        Ok(Self { client })
    }

    /// Reutiliza um cliente KurrentDB já configurado.
    ///
    /// Útil para customizações avançadas de TLS, autenticação ou runtime.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Acesso ao cliente oficial subjacente.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Consome o wrapper devolvendo o cliente oficial.
    pub fn into_inner(self) -> Client {
        self.client
    }

    /// Verifica a conectividade consultando as informações do servidor.
    ///
    /// Operação leve, usada no bootstrap e em health checks.
    pub async fn ping(&self) -> Result<(), KurrentDbError> {
        debug!("verificando conexão com KurrentDB");
        match self.client.server_info().await {
            Ok(info) => {
                info!(
                    server_version = %info.version(),
                    "conexão com KurrentDB estabelecida"
                );
                Ok(())
            }
            Err(error) => {
                let mapped = map_connection_error(error);
                tracing::warn!(error = %mapped, "falha ao conectar ao KurrentDB");
                Err(mapped)
            }
        }
    }
}

/// Converte falhas do cliente em erros tipados do adapter.
pub(crate) fn map_connection_error(error: kurrentdb::Error) -> KurrentDbError {
    match error {
        kurrentdb::Error::ConnectionClosed
        | kurrentdb::Error::GrpcConnectionError(_)
        | kurrentdb::Error::NotLeaderException(_)
        | kurrentdb::Error::DeadlineExceeded => KurrentDbError::Connection(error.to_string()),
        other => KurrentDbError::Storage(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuracoes_vazias_nao_criam_cliente() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let result = runtime.block_on(KurrentDbClient::connect("  "));
        assert!(matches!(result, Err(KurrentDbError::Configuration(_))));
    }

    #[test]
    fn connection_string_invalida_vira_erro_tipado() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let result = runtime.block_on(KurrentDbClient::connect("nao-e-uma-uri"));
        assert!(matches!(
            result,
            Err(KurrentDbError::InvalidConnectionString(_))
        ));
    }
}
