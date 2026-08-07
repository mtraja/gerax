use std::sync::Arc;

use async_trait::async_trait;
use gerax_websocket::websocket::websocket::WebSocketServer;
use gerax_websocket::websocket::handler::{WsHandler, ServerError, WsContext};
use gerax_websocket::websocket::message::WsMessage;
use gerax_websocket::websocket::graphql::{GraphQLClientMessage, GraphQLDataPayload, GraphQLErrorPayload, GraphQLServerMessage};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    Executor, GraphqlError, GraphqlRequest, Resolver,
};

pub type WsResult<T = ()> = Result<T, ServerError>;

struct GraphQLWsHandler<State> {
    manager: Arc<SubscriptionManager<State>>,
}

#[async_trait]
pub trait Subscription<State>: Send + Sync + 'static {
    async fn start(&self, addr: &str) -> Result<(), GraphqlError>;
    async fn stop(&self) -> Result<(), GraphqlError>;
    fn register_resolver(&self, field_name: &str, resolver: Arc<dyn Resolver<State>>);
}

#[async_trait]
impl<State> WsHandler<State> for GraphQLWsHandler<State>
where
    State: Send + Sync + 'static,
{
    async fn on_open(&self, ctx: WsContext<State>) -> WsResult<()> {
        let ack = GraphQLServerMessage::ConnectionAck;
        let text = serde_json::to_string(&ack).map_err(|e| ServerError::HandlerError(e.to_string()))?;
        ctx.send(WsMessage::Text(text)).await?;
        Ok(())
    }

    async fn on_message(&self, ctx: WsContext<State>, msg: WsMessage) -> WsResult<()> {
        let text = match msg {
            WsMessage::Text(text) => text,
            _ => return Ok(()),
        };

        let client_msg: GraphQLClientMessage = serde_json::from_str(&text)
            .map_err(|e| ServerError::HandlerError(e.to_string()))?;

        match client_msg {
            GraphQLClientMessage::ConnectionInit => {
                let ack = GraphQLServerMessage::ConnectionAck;
                let text = serde_json::to_string(&ack).map_err(|e| ServerError::HandlerError(e.to_string()))?;
                ctx.send(WsMessage::Text(text)).await?;
            }
            GraphQLClientMessage::Start { id, payload } => {
                let variables = match payload.variables {
                    Some(Value::Object(map)) => Some(map),
                    None => None,
                    _ => None,
                };

                let executor = self.manager.executor();
                let request = GraphqlRequest {
                    query: payload.query,
                    variables,
                    operation_name: payload.operation_name,
                    extensions: None,
                };

                match executor.execute(request, ctx.state().as_ref()).await {
                    Ok(response) => {
                        let data = response.data.unwrap_or(Value::Null);
                        let server_msg = GraphQLServerMessage::Data {
                            id: id.clone(),
                            payload: GraphQLDataPayload { data },
                        };
                        let text = serde_json::to_string(&server_msg).map_err(|e| ServerError::HandlerError(e.to_string()))?;
                        ctx.send(WsMessage::Text(text)).await?;
                    }
                    Err(err) => {
                        let server_msg = GraphQLServerMessage::Error {
                            id,
                            payload: GraphQLErrorPayload {
                                errors: vec![Value::String(err.to_string())],
                            },
                        };
                        let text = serde_json::to_string(&server_msg).map_err(|e| ServerError::HandlerError(e.to_string()))?;
                        ctx.send(WsMessage::Text(text)).await?;
                    }
                }
            }
            GraphQLClientMessage::Stop { id } => {
                let server_msg = GraphQLServerMessage::Complete { id };
                let text = serde_json::to_string(&server_msg).map_err(|e| ServerError::HandlerError(e.to_string()))?;
                ctx.send(WsMessage::Text(text)).await?;
            }
        }

        Ok(())
    }
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

    /// Acesso ao executor GraphQL.
    pub fn executor(&self) -> &Arc<dyn Executor<State>> {
        &self._executor
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
    state: Arc<State>,
    server: Arc<Mutex<Option<WebSocketServer<State>>>>,
}

impl<State: 'static> WebSocketSubscriptionAdapter<State> {
    /// Cria um novo adapter WebSocket para subscriptions.
    pub fn new(manager: Arc<SubscriptionManager<State>>, state: Arc<State>) -> Self {
        Self {
            manager,
            state,
            server: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl<State: Send + Sync + 'static> Subscription<State> for WebSocketSubscriptionAdapter<State> {
    async fn start(&self, addr: &str) -> Result<(), GraphqlError> {
        let handler = Arc::new(GraphQLWsHandler {
            manager: Arc::clone(&self.manager),
        });

        let server = WebSocketServer::new(
            addr.parse::<std::net::SocketAddr>().map_err(|e| GraphqlError::Execution(e.to_string()))?,
            Arc::clone(&self.state),
            handler,
        );

        let server_clone = server.clone();
        tokio::spawn(async move {
            let _ = server_clone.run().await;
        });

        let mut guard = self.server.lock().await;
        *guard = Some(server);

        Ok(())
    }

    async fn stop(&self) -> Result<(), GraphqlError> {
        let guard = self.server.lock().await;
        if let Some(ref server) = *guard {
            server.stop().await.map_err(|e| GraphqlError::Execution(e.to_string()))?;
        }
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

    use super::{Subscription, SubscriptionManager, WebSocketSubscriptionAdapter};
    use crate::{Executor, GraphqlError, GraphqlRequest, GraphqlResponse, Resolver};

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
        let adapter = WebSocketSubscriptionAdapter::new(manager, Arc::new(()));

        assert!(adapter.start("127.0.0.1:0").await.is_ok());
        assert!(adapter.stop().await.is_ok());
    }
}
