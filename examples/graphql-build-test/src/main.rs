use async_graphql::{Object, SimpleObject, Subscription};
use async_stream::stream;
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use gerax_actix::{ActixHttpServerBuilder, HttpServerBuilder};
use gerax_http::server::HttpServer;
use gerax_graphql::{DefaultExecutor, GraphqlHandler, Schema};
use gerax_http::routing::Router;
use uuid::Uuid;

#[derive(Debug, Clone, SimpleObject)]
struct Message {
    id: String,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Default)]
struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn hello(&self) -> &str {
        "world"
    }
}

#[derive(Default)]
struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_message(&self) -> Message {
        Message {
            id: Uuid::new_v4().to_string(),
            content: "hello".to_string(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Default)]
struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn message_created(&self) -> impl Stream<Item = Message> {
        stream! { yield Message { id: "1".into(), content: "test".into(), created_at: Utc::now() }; }
    }
}

fn main() {
    let schema = Schema::builder()
        .query(QueryRoot)
        .mutation(MutationRoot)
        .subscription(SubscriptionRoot)
        .finish()
        .expect("failed to build schema");

    let executor = DefaultExecutor::<(), _, _, _>::new(schema);
    let handler = GraphqlHandler::new(executor);

    let router = Router::new().post("/graphql", handler);

    let server = ActixHttpServerBuilder::new(())
        .route(router)
        .build()
        .expect("failed to build server");

    println!("Starting server...");

    actix_web::rt::System::new().block_on(async move {
        if let Err(err) = server.run().await {
            eprintln!("server error: {err}");
        }
    });
}
