use std::sync::Arc;

use async_trait::async_trait;
use gerax_websocket::websocket::graphql::{
    GraphQLClientMessage, GraphQLDataPayload, GraphQLErrorPayload, GraphQLServerMessage,
};
use gerax_websocket::websocket::handler::{ServerError, WsContext, WsHandler};
use gerax_websocket::websocket::message::WsMessage;
use gerax_websocket::websocket::websocket::WebSocketServer;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{Executor, GraphqlError, GraphqlRequest, Resolver, context::GraphqlContext};

pub type WsResult<T = ()> = Result<T, ServerError>;

struct GraphQLWsHandler<State> {
    executor: Arc<dyn Executor<State>>,
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
        let text =
            serde_json::to_string(&ack).map_err(|e| ServerError::HandlerError(e.to_string()))?;
        ctx.send(WsMessage::Text(text)).await?;
        Ok(())
    }

    async fn on_message(&self, ctx: WsContext<State>, msg: WsMessage) -> WsResult<()> {
        let text = match msg {
            WsMessage::Text(text) => text,
            _ => return Ok(()),
        };

        let client_msg: GraphQLClientMessage =
            serde_json::from_str(&text).map_err(|e| ServerError::HandlerError(e.to_string()))?;

        match client_msg {
            GraphQLClientMessage::ConnectionInit => {
                let ack = GraphQLServerMessage::ConnectionAck;
                let text = serde_json::to_string(&ack)
                    .map_err(|e| ServerError::HandlerError(e.to_string()))?;
                ctx.send(WsMessage::Text(text)).await?;
            }
            GraphQLClientMessage::Start { id, payload } => {
                let variables = match payload.variables {
                    Some(Value::Object(map)) => Some(map),
                    None => None,
                    _ => None,
                };

                let executor = &self.executor;
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
                        let text = serde_json::to_string(&server_msg)
                            .map_err(|e| ServerError::HandlerError(e.to_string()))?;
                        ctx.send(WsMessage::Text(text)).await?;
                    }
                    Err(err) => {
                        let server_msg = GraphQLServerMessage::Error {
                            id,
                            payload: GraphQLErrorPayload {
                                errors: vec![Value::String(err.to_string())],
                            },
                        };
                        let text = serde_json::to_string(&server_msg)
                            .map_err(|e| ServerError::HandlerError(e.to_string()))?;
                        ctx.send(WsMessage::Text(text)).await?;
                    }
                }
            }
            GraphQLClientMessage::Stop { id } => {
                let server_msg = GraphQLServerMessage::Complete { id };
                let text = serde_json::to_string(&server_msg)
                    .map_err(|e| ServerError::HandlerError(e.to_string()))?;
                ctx.send(WsMessage::Text(text)).await?;
            }
        }

        Ok(())
    }
}

/// Gerenciador de subscriptions ativas.
pub struct SubscriptionManager<State> {
    subscriptions: Arc<tokio::sync::RwLock<Vec<ActiveSubscription<State>>>>,
}

struct ActiveSubscription<State> {
    field_name: String,
    resolver: Arc<dyn Resolver<State>>,
}

impl<State: 'static> SubscriptionManager<State> {
    /// Cria um novo gerenciador de subscriptions.
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(tokio::sync::RwLock::new(Vec::new())),
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
        context: &GraphqlContext<State>,
    ) -> Result<Value, GraphqlError> {
        let subs = self.subscriptions.read().await;
        if let Some(sub) = subs.iter().find(|s| s.field_name == field_name) {
            sub.resolver.resolve(context).await
        } else {
            Err(GraphqlError::Execution(format!(
                "subscription field '{}' not found",
                field_name
            )))
        }
    }
}

impl<State: 'static> Default for SubscriptionManager<State> {
    fn default() -> Self {
        Self::new()
    }
}

/// Adapter WebSocket para subscriptions GraphQL.
pub struct WebSocketSubscriptionAdapter<State> {
    manager: Arc<SubscriptionManager<State>>,
    state: Arc<State>,
    executor: Arc<dyn Executor<State>>,
    server: Arc<Mutex<Option<WebSocketServer<State>>>>,
}

impl<State: 'static> WebSocketSubscriptionAdapter<State> {
    /// Cria um novo adapter WebSocket para subscriptions.
    pub fn new(
        manager: Arc<SubscriptionManager<State>>,
        state: Arc<State>,
        executor: Arc<dyn Executor<State>>,
    ) -> Self {
        Self {
            manager,
            state,
            executor,
            server: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl<State: Send + Sync + 'static> Subscription<State> for WebSocketSubscriptionAdapter<State> {
    async fn start(&self, addr: &str) -> Result<(), GraphqlError> {
        let handler = Arc::new(GraphQLWsHandler {
            executor: Arc::clone(&self.executor),
        });

        let server = WebSocketServer::new(
            addr.parse::<std::net::SocketAddr>()
                .map_err(|e| GraphqlError::Execution(e.to_string()))?,
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
            server
                .stop()
                .await
                .map_err(|e| GraphqlError::Execution(e.to_string()))?;
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
    use crate::{
        Executor, GraphqlError, GraphqlRequest, GraphqlResponse, Resolver, context::GraphqlContext,
    };
    use gerax_http::routing::{Context, HttpMethod, Request};

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
        async fn resolve(&self, _context: &GraphqlContext<()>) -> Result<Value, GraphqlError> {
            Ok(json!({ "id": "event-1" }))
        }
    }

    #[tokio::test]
    async fn manager_resolves_registered_subscriptions() {
        let manager = SubscriptionManager::new();
        manager
            .register("eventCreated".to_string(), Arc::new(ValueResolver))
            .await;

        let context: GraphqlContext<()> = Context::new(
            Arc::new(()),
            Request::new(HttpMethod::Get, "/graphql".into(), Vec::new()),
        );

        assert_eq!(
            manager.resolve("eventCreated", &context).await,
            Ok(json!({ "id": "event-1" }))
        );
        assert!(matches!(
            manager.resolve("missing", &context).await,
            Err(GraphqlError::Execution(_))
        ));
    }

    #[tokio::test]
    async fn websocket_adapter_starts_and_stops() {
        let manager = Arc::new(SubscriptionManager::new());
        let adapter =
            WebSocketSubscriptionAdapter::new(manager, Arc::new(()), Arc::new(NoopExecutor));

        assert!(adapter.start("127.0.0.1:0").await.is_ok());
        assert!(adapter.stop().await.is_ok());
    }
}
