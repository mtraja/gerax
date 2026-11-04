//! Consumidor de Persistent Subscriptions.
//!
//! [`KurrentDbSubscription`] conecta a um grupo **já existente** e
//! entrega [`SubscriptionEvent`]s para a camada superior. A criação do
//! grupo é uma operação administrativa explícita do
//! [`SubscriptionManager`](super::manager::SubscriptionManager) — o
//! consumidor nunca cria grupos automaticamente.
//!
//! ## Uso
//!
//! ```ignore
//! use gerax_kurrentdb::{KurrentDbClient, SubscriptionConfig, KurrentDbSubscription};
//! use gerax_kurrentdb::subscription::SubscriptionAck;
//!
//! let client = KurrentDbClient::connect("kurrentdb://localhost:2113?tls=false").await?;
//! let subscription = KurrentDbSubscription::new(
//!     client,
//!     SubscriptionConfig::new("Aluno-aluno-1", "relay"),
//! )?;
//!
//! // Uma sessão entrega eventos e permite ACK/NACK explícitos.
//! let mut session = subscription.subscribe().await?;
//! let event = session.next_event().await?;
//! session.ack(&event, SubscriptionAck::Ack).await?;
//! ```

use std::future::Future;

use kurrentdb::{PersistentSubscription, PersistentSubscriptionEvent};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use gerax_es::event::EventId;

use crate::client::KurrentDbClient;
use crate::config::KurrentDbConfig;

use super::ack::SubscriptionAck;
use super::config::SubscriptionConfig;
use super::error::SubscriptionError;
use super::event::SubscriptionEvent;

/// Consumidor de uma Persistent Subscription.
///
/// O mesmo cliente KurrentDB pode ser compartilhado com o
/// [`KurrentDbEventStore`](crate::event_store::KurrentDbEventStore); as
/// duas funcionalidades permanecem independentes.
#[derive(Clone)]
pub struct KurrentDbSubscription {
    client: KurrentDbClient,
    config: SubscriptionConfig,
}

impl KurrentDbSubscription {
    /// Cria o consumidor a partir de um cliente e de uma configuração
    /// já validada.
    pub fn new(
        client: KurrentDbClient,
        config: SubscriptionConfig,
    ) -> Result<Self, SubscriptionError> {
        config.validate()?;
        Ok(Self { client, config })
    }

    /// Cria o consumidor a partir da configuração de conexão, usando a
    /// subscription padrão nela declarada.
    pub async fn from_config(config: &KurrentDbConfig) -> Result<Self, SubscriptionError> {
        let subscription = config.subscription.clone().ok_or_else(|| {
            SubscriptionError::Configuration(
                "KurrentDbConfig does not contain a `subscription` section".into(),
            )
        })?;
        let client = KurrentDbClient::from_config(config)
            .await
            .map_err(|error| SubscriptionError::Connection(error.to_string()))?;
        Self::new(client, subscription)
    }

    /// Conecta ao KurrentDB e cria o consumidor.
    pub async fn connect(
        connection_string: &str,
        config: SubscriptionConfig,
    ) -> Result<Self, SubscriptionError> {
        let client = KurrentDbClient::connect(connection_string)
            .await
            .map_err(|error| SubscriptionError::Connection(error.to_string()))?;
        Self::new(client, config)
    }

    /// Configuração do grupo consumido.
    pub fn config(&self) -> &SubscriptionConfig {
        &self.config
    }

    /// Cliente compartilhado subjacente.
    pub fn client(&self) -> &KurrentDbClient {
        &self.client
    }

    /// Conecta ao grupo configurado e devolve uma sessão de consumo.
    ///
    /// O grupo precisa existir. Use
    /// [`SubscriptionManager::ensure`](super::manager::SubscriptionManager::ensure)
    /// no bootstrap da aplicação para criá-lo de forma idempotente.
    pub async fn subscribe(&self) -> Result<SubscriptionSession, SubscriptionError> {
        let options = self.config.to_subscribe_options();
        let group = self.config.group.clone();

        let inner = if self.config.is_all_stream() {
            self.client
                .client()
                .subscribe_to_persistent_subscription_to_all(&group, &options)
                .await
        } else {
            self.client
                .client()
                .subscribe_to_persistent_subscription(self.config.stream.as_str(), &group, &options)
                .await
        };

        match inner {
            Ok(inner) => {
                info!(
                    stream = %self.config.stream,
                    subscription_group = %self.config.group,
                    "conectado à persistent subscription"
                );
                Ok(SubscriptionSession {
                    inner,
                    config: self.config.clone(),
                })
            }
            Err(error) => {
                let mapped = map_subscription_error(error, &self.config);
                warn!(
                    stream = %self.config.stream,
                    subscription_group = %self.config.group,
                    error = %mapped,
                    "falha ao conectar à persistent subscription"
                );
                Err(mapped)
            }
        }
    }

    /// Consome eventos continuamente, reconectando após falhas
    /// transitórias.
    ///
    /// O handler recebe cada evento e devolve a ação de confirmação. A
    /// conexão é recriada automaticamente enquanto o erro for
    /// retentável. Para encerrar o loop, faça o handler devolver um
    /// erro permanente ou remova a subscription.
    pub async fn run<F, Fut>(&self, mut handler: F) -> Result<(), SubscriptionError>
    where
        F: FnMut(SubscriptionEvent) -> Fut + Send,
        Fut: Future<Output = Result<SubscriptionAck, SubscriptionError>> + Send,
    {
        loop {
            let mut session = match self.subscribe().await {
                Ok(session) => session,
                Err(error) if error.is_retryable() => {
                    warn!(
                        error = %error,
                        delay_ms = self.config.reconnect_delay_ms,
                        "reconectando à persistent subscription"
                    );
                    sleep(self.config.reconnect_delay()).await;
                    continue;
                }
                Err(error) => return Err(error),
            };

            let reconnect = loop {
                match session.next_event().await {
                    Ok(event) => {
                        let action = match handler(event.clone()).await {
                            Ok(action) => action,
                            Err(error) if error.is_retryable() => SubscriptionAck::Retry,
                            Err(error) => {
                                error!(
                                    error = %error,
                                    "handler reportou falha permanente"
                                );
                                SubscriptionAck::Park
                            }
                        };

                        if let Err(error) = session.ack(&event, action).await {
                            warn!(error = %error, "falha ao confirmar evento");
                            break true;
                        }
                    }
                    Err(error) if error.is_retryable() => {
                        warn!(error = %error, "conexão da subscription perdida");
                        break true;
                    }
                    Err(SubscriptionError::Closed) => return Ok(()),
                    Err(error) => return Err(error),
                }
            };

            if reconnect {
                debug!(
                    delay_ms = self.config.reconnect_delay_ms,
                    "reconectando após perda de conexão"
                );
                sleep(self.config.reconnect_delay()).await;
            }
        }
    }
}

/// Sessão ativa de consumo de um grupo.
///
/// A sessão é deliberadamente de baixo nível: a `gerax-eda` controla o
/// processamento e só então chama [`SubscriptionSession::ack`].
pub struct SubscriptionSession {
    inner: PersistentSubscription,
    config: SubscriptionConfig,
}

impl SubscriptionSession {
    /// Configuração do grupo desta sessão.
    pub fn config(&self) -> &SubscriptionConfig {
        &self.config
    }

    /// Aguarda o próximo evento.
    ///
    /// Confirmações internas de checkpoint (`Confirmed`) são ignoradas.
    pub async fn next_event(&mut self) -> Result<SubscriptionEvent, SubscriptionError> {
        loop {
            match self.inner.next_subscription_event().await {
                Ok(PersistentSubscriptionEvent::EventAppeared { retry_count, event }) => {
                    let record = event.get_original_event();
                    let event = SubscriptionEvent::from_recorded(
                        record,
                        retry_count.min(u32::MAX as usize) as u32,
                        event.commit_position,
                    )?;

                    debug!(
                        event_id = %event.id.as_uuid(),
                        event_type = %event.event_type,
                        aggregate_id = %event.aggregate_id,
                        stream_revision = event.stream_revision,
                        subscription_group = %self.config.group,
                        retry_count = event.retry_count,
                        "evento recebido da persistent subscription"
                    );

                    return Ok(event);
                }
                Ok(PersistentSubscriptionEvent::Confirmed(_)) => continue,
                Err(error) => return Err(map_subscription_error(error, &self.config)),
            }
        }
    }

    /// Confirma um evento com a ação indicada.
    ///
    /// O ACK só deve ser enviado após a camada superior concluir o
    /// processamento — no Event Relay, somente após a confirmação de
    /// publicação no NATS JetStream.
    pub async fn ack(
        &mut self,
        event: &SubscriptionEvent,
        action: SubscriptionAck,
    ) -> Result<(), SubscriptionError> {
        self.ack_id(event.id.clone(), action, "gerax subscription")
            .await
    }

    /// Confirma por identificador (útil quando o consumidor guarda
    /// apenas o `EventId`).
    pub async fn ack_id(
        &mut self,
        event_id: EventId,
        action: SubscriptionAck,
        reason: impl AsRef<str>,
    ) -> Result<(), SubscriptionError> {
        let id = event_id.as_uuid();

        let result = match action {
            SubscriptionAck::Ack => self.inner.ack_ids([id]).await,
            action => {
                let nak = action
                    .nak_action()
                    .expect("NACK action required for non-ACK results");
                self.inner.nack_ids([id], nak, reason.as_ref()).await
            }
        };

        result.map_err(|error| {
            let mapped = SubscriptionError::acknowledgement(action, error.to_string());
            warn!(
                event_id = %id,
                action = ?action,
                error = %mapped,
                subscription_group = %self.config.group,
                "falha ao confirmar evento"
            );
            mapped
        })
    }

    /// Encerra a sessão (o drop fecha o stream gRPC).
    pub fn close(self) {
        drop(self.inner);
    }
}

/// Traduz erros nativos do cliente para [`SubscriptionError`].
fn map_subscription_error(
    error: kurrentdb::Error,
    config: &SubscriptionConfig,
) -> SubscriptionError {
    match error {
        kurrentdb::Error::ResourceNotFound => SubscriptionError::GroupNotFound {
            stream: config.stream.clone(),
            group: config.group.clone(),
        },
        kurrentdb::Error::ResourceAlreadyExists => SubscriptionError::GroupAlreadyExists {
            stream: config.stream.clone(),
            group: config.group.clone(),
        },
        kurrentdb::Error::ConnectionClosed
        | kurrentdb::Error::GrpcConnectionError(_)
        | kurrentdb::Error::NotLeaderException(_)
        | kurrentdb::Error::DeadlineExceeded
        | kurrentdb::Error::ServerError(_)
        | kurrentdb::Error::Grpc { .. } => SubscriptionError::Connection(error.to_string()),
        kurrentdb::Error::IllegalStateError(message) => SubscriptionError::Receive(message),
        other => SubscriptionError::Receive(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::subscription::config::{ALL_STREAM, SubscriptionConfig};

    #[test]
    fn configuracao_invalida_nao_cria_consumidor() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = runtime
            .block_on(KurrentDbClient::connect(
                "kurrentdb://localhost:2113?tls=false",
            ))
            .unwrap();
        let result = KurrentDbSubscription::new(client, SubscriptionConfig::new("", "relay"));
        assert!(matches!(result, Err(SubscriptionError::Configuration(_))));
    }

    #[test]
    fn erro_de_grupo_inexistente_preserva_stream_e_grupo() {
        let config = SubscriptionConfig::all("relay");
        let error = map_subscription_error(kurrentdb::Error::ResourceNotFound, &config);
        match error {
            SubscriptionError::GroupNotFound { stream, group } => {
                assert_eq!(stream, ALL_STREAM);
                assert_eq!(group, "relay");
            }
            other => panic!("esperava GroupNotFound, obtido {other:?}"),
        }
    }

    #[test]
    fn erro_de_conexao_e_retentavel() {
        let config = SubscriptionConfig::all("relay");
        let error = map_subscription_error(kurrentdb::Error::ConnectionClosed, &config);
        assert!(error.is_retryable());
    }
}
