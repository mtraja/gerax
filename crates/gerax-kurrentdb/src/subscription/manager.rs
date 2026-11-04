//! Gerenciamento administrativo de grupos de Persistent Subscription.
//!
//! Criar, atualizar e remover grupos são operações **explícitas**. O
//! consumidor nunca cria grupos silenciosamente: isso evita que cada
//! instância de uma aplicação recrie ou reescreva a configuração do
//! grupo no bootstrap.
//!
//! Múltiplas instâncias que desejam processamento concorrente devem
//! usar o **mesmo** grupo; grupos diferentes possuem checkpoints
//! independentes.

use kurrentdb::{
    DeletePersistentSubscriptionOptions, GetPersistentSubscriptionInfoOptions,
    ListPersistentSubscriptionsOptions, PersistentSubscriptionInfo, ReplayParkedMessagesOptions,
};
use tracing::{info, warn};

use crate::client::KurrentDbClient;
use crate::config::KurrentDbConfig;

use super::config::{ALL_STREAM, SubscriptionConfig};
use super::error::SubscriptionError;

/// Estado retornado por [`SubscriptionManager::ensure`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupState {
    /// O grupo foi criado nesta chamada.
    Created,
    /// O grupo já existia e foi preservado.
    Existing,
}

/// Visão independente do cliente sobre um grupo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionInfo {
    /// Stream de origem (`$all` ou `{aggregate_type}-{aggregate_id}`).
    pub stream: String,
    /// Nome do grupo.
    pub group: String,
    /// Status reportado pelo servidor.
    pub status: String,
    /// Quantidade de consumidores conectados.
    pub connections: usize,
    /// Total de itens processados.
    pub total_items: usize,
    /// Mensagens em voo (entregues e ainda não confirmadas).
    pub total_in_flight: usize,
    /// Mensagens na fila de park.
    pub parked_message_count: usize,
    /// Última revisão de stream com checkpoint, quando aplicável.
    pub last_checkpointed_revision: Option<u64>,
    /// Última posição global com checkpoint, quando aplicável.
    pub last_checkpointed_position: Option<String>,
}

/// Gerenciador de grupos de Persistent Subscription.
#[derive(Clone)]
pub struct SubscriptionManager {
    client: KurrentDbClient,
}

impl SubscriptionManager {
    /// Cria o gerenciador sobre um cliente compartilhado.
    pub fn new(client: KurrentDbClient) -> Self {
        Self { client }
    }

    /// Conecta ao KurrentDB e cria o gerenciador.
    pub async fn connect(connection_string: &str) -> Result<Self, SubscriptionError> {
        let client = KurrentDbClient::connect(connection_string)
            .await
            .map_err(|error| SubscriptionError::Connection(error.to_string()))?;
        Ok(Self::new(client))
    }

    /// Cria o gerenciador a partir da configuração do adapter.
    pub async fn from_config(config: &KurrentDbConfig) -> Result<Self, SubscriptionError> {
        let client = KurrentDbClient::from_config(config)
            .await
            .map_err(|error| SubscriptionError::Connection(error.to_string()))?;
        Ok(Self::new(client))
    }

    /// Cliente subjacente.
    pub fn client(&self) -> &KurrentDbClient {
        &self.client
    }

    /// Cria um grupo.
    ///
    /// Retorna [`SubscriptionError::GroupAlreadyExists`] quando o grupo
    /// já existe. Use [`Self::ensure`] para uma criação idempotente.
    pub async fn create(&self, config: &SubscriptionConfig) -> Result<(), SubscriptionError> {
        config.validate()?;

        let result = if config.is_all_stream() {
            let options = config.to_all_options()?;
            self.client
                .client()
                .create_persistent_subscription_to_all(&config.group, &options)
                .await
        } else {
            let options = config.to_stream_options()?;
            self.client
                .client()
                .create_persistent_subscription(config.stream.as_str(), &config.group, &options)
                .await
        };

        match result {
            Ok(()) => {
                info!(
                    stream = %config.stream,
                    subscription_group = %config.group,
                    "grupo de persistent subscription criado"
                );
                Ok(())
            }
            Err(error) => Err(map_group_error(error, config)),
        }
    }

    /// Garante que o grupo exista, sem alterar grupos já existentes.
    pub async fn ensure(
        &self,
        config: &SubscriptionConfig,
    ) -> Result<GroupState, SubscriptionError> {
        if self.exists(&config.stream, &config.group).await? {
            return Ok(GroupState::Existing);
        }

        match self.create(config).await {
            Ok(()) => Ok(GroupState::Created),
            Err(SubscriptionError::GroupAlreadyExists { .. }) => Ok(GroupState::Existing),
            Err(error) => Err(error),
        }
    }

    /// Atualiza a configuração de um grupo existente.
    pub async fn update(&self, config: &SubscriptionConfig) -> Result<(), SubscriptionError> {
        config.validate()?;

        let result = if config.is_all_stream() {
            let options = config.to_all_options()?;
            self.client
                .client()
                .update_persistent_subscription_to_all(&config.group, &options)
                .await
        } else {
            let options = config.to_stream_options()?;
            self.client
                .client()
                .update_persistent_subscription(config.stream.as_str(), &config.group, &options)
                .await
        };

        match result {
            Ok(()) => {
                info!(
                    stream = %config.stream,
                    subscription_group = %config.group,
                    "grupo de persistent subscription atualizado"
                );
                Ok(())
            }
            Err(error) => Err(map_group_error(error, config)),
        }
    }

    /// Remove um grupo (ação explícita).
    pub async fn delete(&self, stream: &str, group: &str) -> Result<(), SubscriptionError> {
        let options = DeletePersistentSubscriptionOptions::default();

        let result = if stream == ALL_STREAM {
            self.client
                .client()
                .delete_persistent_subscription_to_all(group, &options)
                .await
        } else {
            self.client
                .client()
                .delete_persistent_subscription(stream, group, &options)
                .await
        };

        match result {
            Ok(()) => {
                warn!(
                    stream = %stream,
                    subscription_group = %group,
                    "grupo de persistent subscription removido"
                );
                Ok(())
            }
            Err(kurrentdb::Error::ResourceNotFound) => Err(SubscriptionError::GroupNotFound {
                stream: stream.to_string(),
                group: group.to_string(),
            }),
            Err(error) => Err(SubscriptionError::GroupManagement(format!(
                "delete group `{group}` on `{stream}` failed: {error}"
            ))),
        }
    }

    /// Verifica se um grupo existe.
    pub async fn exists(&self, stream: &str, group: &str) -> Result<bool, SubscriptionError> {
        match self.info(stream, group).await {
            Ok(_) => Ok(true),
            Err(SubscriptionError::GroupNotFound { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Consulta a informação de um grupo.
    pub async fn info(
        &self,
        stream: &str,
        group: &str,
    ) -> Result<SubscriptionInfo, SubscriptionError> {
        let options = GetPersistentSubscriptionInfoOptions::default();

        let result = if stream == ALL_STREAM {
            self.client
                .client()
                .get_persistent_subscription_info_to_all(group, &options)
                .await
                .map(|info| map_info(info, stream))
        } else {
            self.client
                .client()
                .get_persistent_subscription_info(stream, group, &options)
                .await
                .map(|info| map_info(info, stream))
        };

        result.map_err(|error| map_group_lookup_error(error, stream, group))
    }

    /// Lista os grupos de uma stream (ou de `$all`).
    pub async fn list(&self, stream: &str) -> Result<Vec<SubscriptionInfo>, SubscriptionError> {
        let options = ListPersistentSubscriptionsOptions::default();

        let result = if stream == ALL_STREAM {
            self.client
                .client()
                .list_persistent_subscriptions_to_all(&options)
                .await
                .map(|groups| {
                    groups
                        .into_iter()
                        .map(|group| map_info(group, stream))
                        .collect()
                })
        } else {
            self.client
                .client()
                .list_persistent_subscriptions_for_stream(stream, &options)
                .await
                .map(|groups| {
                    groups
                        .into_iter()
                        .map(|group| map_info(group, stream))
                        .collect()
                })
        };

        result.map_err(|error| {
            SubscriptionError::GroupManagement(format!("list groups on `{stream}` failed: {error}"))
        })
    }

    /// Lista todos os grupos do cluster.
    pub async fn list_all(&self) -> Result<Vec<SubscriptionInfo>, SubscriptionError> {
        let options = ListPersistentSubscriptionsOptions::default();
        self.client
            .client()
            .list_all_persistent_subscriptions(&options)
            .await
            .map(|groups| {
                groups
                    .into_iter()
                    .map(|group| {
                        let stream = group.event_source.clone();
                        map_info(group, &stream)
                    })
                    .collect()
            })
            .map_err(|error| {
                SubscriptionError::GroupManagement(format!("list all groups failed: {error}"))
            })
    }

    /// Reencaminha mensagens parkeadas para o grupo.
    pub async fn replay_parked(
        &self,
        stream: &str,
        group: &str,
        stop_at: Option<usize>,
    ) -> Result<(), SubscriptionError> {
        let mut options = ReplayParkedMessagesOptions::default();
        if let Some(stop_at) = stop_at {
            options = options.stop_at(stop_at);
        }

        let result = if stream == ALL_STREAM {
            self.client
                .client()
                .replay_parked_messages_to_all(group, &options)
                .await
        } else {
            self.client
                .client()
                .replay_parked_messages(stream, group, &options)
                .await
        };

        result.map_err(|error| {
            SubscriptionError::GroupManagement(format!(
                "replay parked messages for `{group}` on `{stream}` failed: {error}"
            ))
        })
    }
}

fn map_info<A>(info: PersistentSubscriptionInfo<A>, stream: &str) -> SubscriptionInfo {
    SubscriptionInfo {
        stream: if stream.is_empty() {
            info.event_source.clone()
        } else {
            stream.to_string()
        },
        group: info.group_name,
        status: info.status,
        connections: info.connections.len(),
        total_items: info.stats.total_items,
        total_in_flight: info.stats.total_in_flight_messages,
        parked_message_count: info.stats.parked_message_count,
        last_checkpointed_revision: info.stats.last_checkpointed_event_revision,
        last_checkpointed_position: info
            .stats
            .last_checkpointed_position
            .map(|position| position.to_string()),
    }
}

fn map_group_error(error: kurrentdb::Error, config: &SubscriptionConfig) -> SubscriptionError {
    match error {
        kurrentdb::Error::ResourceAlreadyExists => SubscriptionError::GroupAlreadyExists {
            stream: config.stream.clone(),
            group: config.group.clone(),
        },
        kurrentdb::Error::ResourceNotFound => SubscriptionError::GroupNotFound {
            stream: config.stream.clone(),
            group: config.group.clone(),
        },
        other => SubscriptionError::GroupManagement(format!(
            "group `{}` on `{}` failed: {other}",
            config.group, config.stream
        )),
    }
}

fn map_group_lookup_error(error: kurrentdb::Error, stream: &str, group: &str) -> SubscriptionError {
    match error {
        kurrentdb::Error::ResourceNotFound => SubscriptionError::GroupNotFound {
            stream: stream.to_string(),
            group: group.to_string(),
        },
        other => SubscriptionError::GroupManagement(format!(
            "lookup group `{group}` on `{stream}` failed: {other}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::subscription::config::SubscriptionConfig;

    #[test]
    fn informacao_e_mapeada_sem_expor_o_cliente() {
        let info = SubscriptionInfo {
            stream: "$all".into(),
            group: "relay".into(),
            status: "Live".into(),
            connections: 2,
            total_items: 10,
            total_in_flight: 1,
            parked_message_count: 3,
            last_checkpointed_revision: Some(42),
            last_checkpointed_position: None,
        };

        assert_eq!(info.stream, ALL_STREAM);
        assert_eq!(info.connections, 2);
        assert_eq!(info.parked_message_count, 3);
    }

    #[test]
    fn erro_de_grupo_duplicado_preserva_identidade() {
        let config = SubscriptionConfig::new("Aluno-1", "relay");
        let error = map_group_error(kurrentdb::Error::ResourceAlreadyExists, &config);
        match error {
            SubscriptionError::GroupAlreadyExists { stream, group } => {
                assert_eq!(stream, "Aluno-1");
                assert_eq!(group, "relay");
            }
            other => panic!("esperava GroupAlreadyExists, obtido {other:?}"),
        }
    }

    #[test]
    fn erro_de_grupo_inexistente_preserva_identidade() {
        let error = map_group_lookup_error(kurrentdb::Error::ResourceNotFound, "$all", "relay");
        assert!(matches!(error, SubscriptionError::GroupNotFound { .. }));
    }
}
