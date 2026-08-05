//! Benchmarks dos componentes principais do `gerax-graphql`.
//!
//! Execute com `cargo bench -p gerax-graphql`. Defina
//! `GERAX_BENCH_ITERATIONS` para ajustar a quantidade de iterações.

use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use async_graphql::{EmptyMutation, EmptySubscription, Object};
use async_trait::async_trait;
use gerax_graphql::{
    BatchLoader, DataLoader, DefaultExecutor, Executor, GraphqlError, GraphqlRequest,
    QueryResolver, Resolver, Schema, SubscriptionManager,
};
use gerax_http::routing::{Context, HttpMethod, Request};

const DEFAULT_ITERATIONS: usize = 1_000;

struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn answer(&self) -> i32 {
        42
    }
}

struct PassthroughLoader;

#[async_trait]
impl BatchLoader<(), u64, u64> for PassthroughLoader {
    async fn load(
        &self,
        _state: Arc<()>,
        keys: Vec<u64>,
    ) -> Result<HashMap<u64, u64>, GraphqlError> {
        Ok(keys.into_iter().map(|key| (key, key)).collect())
    }
}

struct NoopExecutor;

#[async_trait]
impl Executor<()> for NoopExecutor {
    async fn execute(
        &self,
        _request: GraphqlRequest,
        _state: &(),
    ) -> Result<gerax_graphql::GraphqlResponse, GraphqlError> {
        Ok(gerax_graphql::GraphqlResponse::default())
    }
}

struct SubscriptionResolver;

#[async_trait]
impl Resolver<()> for SubscriptionResolver {
    async fn resolve(
        &self,
        _state: &(),
        _args: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, GraphqlError> {
        Ok(serde_json::Value::Null)
    }
}

fn iterations() -> usize {
    std::env::var("GERAX_BENCH_ITERATIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_ITERATIONS)
}

fn measure(name: &str, iterations: usize, mut operation: impl FnMut()) {
    let started_at = Instant::now();
    for _ in 0..iterations {
        operation();
    }
    let elapsed = started_at.elapsed();
    let nanos_per_iteration = elapsed.as_nanos() / iterations as u128;
    println!("{name}: {nanos_per_iteration} ns/iter ({iterations} iterations)");
}

fn consume_result<T, E>(result: Result<T, E>)
where
    E: std::fmt::Display,
{
    match result {
        Ok(value) => {
            let _ = black_box(value);
        }
        Err(error) => {
            eprintln!("benchmark operation failed: {error}");
            std::process::exit(1);
        }
    }
}

fn build_schema() -> Result<Schema<QueryRoot, EmptyMutation, EmptySubscription>, GraphqlError> {
    Schema::builder()
        .query(QueryRoot)
        .mutation(EmptyMutation)
        .subscription(EmptySubscription)
        .finish()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let iterations = iterations();
    let runtime = tokio::runtime::Runtime::new()?;

    measure("schema_build", iterations, || {
        consume_result(build_schema());
    });

    let schema = build_schema()?;
    let executor = DefaultExecutor::<(), _, _, _>::new(schema);
    let request = GraphqlRequest {
        query: "{ answer }".to_string(),
        ..GraphqlRequest::default()
    };
    measure("executor_execute", iterations, || {
        consume_result(runtime.block_on(executor.execute(request.clone(), &())));
    });

    let resolver = QueryResolver::<()>::new();
    measure("resolver_resolve", iterations, || {
        consume_result(runtime.block_on(resolver.resolve(&(), None)));
    });

    let manager = SubscriptionManager::new(Arc::new(NoopExecutor));
    runtime.block_on(manager.register("eventCreated".to_string(), Arc::new(SubscriptionResolver)));
    measure("subscription_resolve", iterations, || {
        consume_result(runtime.block_on(manager.resolve("eventCreated", &(), None)));
    });

    let context = Context::new(
        Arc::new(()),
        Request::new(HttpMethod::Post, "/graphql".to_string(), Vec::new()),
    );
    let loader = DataLoader::new(PassthroughLoader);
    measure("dataloader_load", iterations, || {
        runtime.block_on(loader.clear_cache());
        consume_result(runtime.block_on(loader.load(&context, 42)));
    });

    Ok(())
}
