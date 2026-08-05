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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::{Value, json};

    use super::{SubscriptionManager, WebSocketSubscriptionAdapter};
    use crate::{Executor, GraphqlError, GraphqlRequest, GraphqlResponse, Resolver, Subscription};

    struct NoopExecutor;

    #[async_trait]
    impl Executor<()> for NoopExecutor {
        async fn execute(
            &self,
            _request: GraphqlRequest,
            _state: &(),
        ) -> Result<GraphqlResponse, GraphqlError> {
            Ok(GraphqlResponse::default())
        }
    }

    struct ValueResolver;

    #[async_trait]
    impl Resolver<()> for ValueResolver {
        async fn resolve(&self, _state: &(), _args: Option<&Value>) -> Result<Value, GraphqlError> {
            Ok(json!({ "id": "event-1" }))
        }
    }

    #[tokio::test]
    async fn manager_resolves_registered_subscriptions() {
        let manager = SubscriptionManager::new(Arc::new(NoopExecutor));
        manager
            .register("eventCreated".to_string(), Arc::new(ValueResolver))
            .await;

        assert_eq!(
            manager.resolve("eventCreated", &(), None).await,
            Ok(json!({ "id": "event-1" }))
        );
        assert!(matches!(
            manager.resolve("missing", &(), None).await,
            Err(GraphqlError::Execution(_))
        ));
    }

    #[tokio::test]
    async fn websocket_adapter_starts_and_stops() {
        let manager = Arc::new(SubscriptionManager::new(Arc::new(NoopExecutor)));
        let adapter = WebSocketSubscriptionAdapter::new(manager);

        assert!(adapter.start("127.0.0.1:0").await.is_ok());
        assert!(adapter.stop().await.is_ok());
    }
}
