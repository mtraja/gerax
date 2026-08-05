use crate::{Executor, GraphqlError, Resolver};
use async_trait::async_trait;
use std::sync::Arc;

/// Trait de abstração para subscriptions GraphQL.
///
/// Independente do framework HTTP.
/// Cada adapter será responsável por conectar ao servidor.
#[async_trait]
pub trait Subscription<State>: Send + Sync + 'static {
    /// Inicia o servidor de subscriptions.
    async fn start(&self, addr: &str) -> Result<(), GraphqlError>;

    /// Para o servidor de subscriptions.
    async fn stop(&self) -> Result<(), GraphqlError>;

    /// Registra um resolver para um campo de subscription.
    fn register_resolver(&self, field_name: &str, resolver: Arc<dyn Resolver<State>>);
}

/// Gerenciador de subscriptions ativas.
pub struct SubscriptionManager<State> {
    subscriptions: Arc<tokio::sync::RwLock<Vec<ActiveSubscription<State>>>>,
    _executor: Arc<dyn Executor<State>>,
}

struct ActiveSubscription<State> {
    field_name: String,
    resolver: Arc<dyn Resolver<State>>,
}

impl<State: 'static> SubscriptionManager<State> {
    /// Cria um novo gerenciador de subscriptions.
    pub fn new(executor: Arc<dyn Executor<State>>) -> Self {
        Self {
            subscriptions: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            _executor: executor,
        }
    }

    /// Registra uma subscription com um resolver.
    pub async fn register(&self, field_name: String, resolver: Arc<dyn Resolver<State>>) {
        let mut subs = self.subscriptions.write().await;
        subs.push(ActiveSubscription {
            field_name,
            resolver,
        });
    }

    /// Resolve uma subscription e retorna o resultado.
    pub async fn resolve(
        &self,
        field_name: &str,
        state: &State,
        args: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, GraphqlError> {
        let subs = self.subscriptions.read().await;
        if let Some(sub) = subs.iter().find(|s| s.field_name == field_name) {
            sub.resolver.resolve(state, args).await
        } else {
            Err(GraphqlError::Execution(format!(
                "subscription field '{}' not found",
                field_name
            )))
        }
    }
}

/// Adapter WebSocket para subscriptions GraphQL.
pub struct WebSocketSubscriptionAdapter<State> {
    manager: Arc<SubscriptionManager<State>>,
}

impl<State: 'static> WebSocketSubscriptionAdapter<State> {
    /// Cria um novo adapter WebSocket para subscriptions.
    pub fn new(manager: Arc<SubscriptionManager<State>>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl<State: Send + Sync + 'static> Subscription<State> for WebSocketSubscriptionAdapter<State> {
    async fn start(&self, _addr: &str) -> Result<(), GraphqlError> {
        Ok(())
    }

    async fn stop(&self) -> Result<(), GraphqlError> {
        Ok(())
    }

    fn register_resolver(&self, field_name: &str, resolver: Arc<dyn Resolver<State>>) {
        let manager = self.manager.clone();
        let field_name = field_name.to_string();
        tokio::spawn(async move {
            manager.register(field_name, resolver).await;
        });
    }
}
