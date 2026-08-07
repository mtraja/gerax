use std::collections::HashMap;

use async_graphql::{Object, SimpleObject, Subscription};
use async_stream::stream;
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use gerax_actix::{ActixHttpServerBuilder, HttpServerBuilder};
use gerax_http::server::HttpServer;
use gerax_graphql::{DefaultExecutor, GraphqlHandler, GraphiQL, Schema};
use gerax_http::routing::{Context as HttpContext, Response, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, SimpleObject)]
pub struct Message {
    pub id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, async_graphql::InputObject)]
pub struct CreateMessageInput {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, SimpleObject)]
pub struct CreateMessagePayload {
    pub message: Message,
}

#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn messages(&self, ctx: &async_graphql::Context<'_>) -> Vec<Message> {
        let state = ctx.data::<AppState>().unwrap();
        state.messages.read().await.values().cloned().collect()
    }

    async fn message(&self, ctx: &async_graphql::Context<'_>, id: String) -> Option<Message> {
        let state = ctx.data::<AppState>().unwrap();
        state.messages.read().await.get(&id).cloned()
    }

    async fn hello(&self) -> &str {
        "world"
    }
}

#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_message(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: CreateMessageInput,
    ) -> CreateMessagePayload {
        let state = ctx.data::<AppState>().unwrap();
        let message = Message {
            id: Uuid::new_v4().to_string(),
            content: input.content,
            created_at: Utc::now(),
        };

        state.messages.write().await.insert(message.id.clone(), message.clone());
        let _ = state.tx.send(message.clone());

        CreateMessagePayload { message }
    }

    async fn delete_message(&self, ctx: &async_graphql::Context<'_>, id: String) -> bool {
        let state = ctx.data::<AppState>().unwrap();
        state.messages.write().await.remove(&id).is_some()
    }
}

#[derive(Default)]
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn message_created(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> impl Stream<Item = Message> {
        let state = ctx.data::<AppState>().unwrap();
        let mut rx = state.tx.subscribe();
        let mut seen: Vec<String> = Vec::new();

        stream! {
            let current = state.messages.read().await.values().cloned().collect::<Vec<_>>();
            for msg in current {
                seen.push(msg.id.clone());
                yield msg;
            }

            while let Ok(msg) = rx.recv().await {
                if !seen.contains(&msg.id) {
                    seen.push(msg.id.clone());
                    yield msg;
                }
            }
        }
    }
}

pub struct AppState {
    pub messages: RwLock<HashMap<String, Message>>,
    pub tx: broadcast::Sender<Message>,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            messages: RwLock::new(HashMap::new()),
            tx,
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = AppState::new();
    let schema = Schema::builder()
        .query(QueryRoot)
        .mutation(MutationRoot)
        .subscription(SubscriptionRoot)
        .finish()
        .expect("failed to build schema");

    let executor = DefaultExecutor::new(schema);
    let handler = GraphqlHandler::new(executor);

    let graphiql_handler = |_ctx: HttpContext<AppState>| async move {
        let ui = GraphiQL::new("/graphql");
        Ok(Response::ok(ui.render()))
    };

    let router = Router::new()
        .post("/graphql", handler)
        .get("/graphiql", graphiql_handler);

    let server = ActixHttpServerBuilder::new(state)
        .route(router)
        .build()
        .expect("failed to build server");

    println!("GraphQL demo running on http://localhost:8080/graphiql");
    println!("GraphQL endpoint at http://localhost:8080/graphql");

    server
        .run()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}
