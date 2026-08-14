//! Exemplo: CRUD gRPC com entidade `User`, repositório em memória,
//! `GrpcService`, `GrpcHandler` e `GrpcClient`.
//!
//! Demonstra:
//! - definição de entidade (`gerax-core::Entity`);
//! - geração de schema Protobuf com `ProtoBuilder`;
//! - serviço gRPC com repositório em memória;
//! - handler HTTP para expor o serviço via `gerax-http`;
//! - cliente gRPC genérico;
//! - configuração de endereço com `GrpcConfig`.
//!
//! Execute com:
//!
//! ```bash
//! cargo run --example crud -p gerax-grpc
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use gerax_codec::JsonCodec;
use gerax_core::Entity;
use gerax_db::{DbError, Repository};
use gerax_grpc::proto::RustType;
use gerax_grpc::{GrpcClient, GrpcConfig, GrpcHandler, GrpcService, ProtoBuilder};
use gerax_http::Router;
use gerax_http::routing::{Context, HttpMethod, Request};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct User {
    id: String,
    name: String,
}

impl Entity for User {
    fn collection_name() -> &'static str {
        "users"
    }

    fn id(&self) -> Option<String> {
        Some(self.id.clone())
    }

    fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

#[derive(Default)]
struct InMemoryRepo {
    users: std::sync::Mutex<Vec<User>>,
}

#[async_trait]
impl Repository<User> for InMemoryRepo {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, DbError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned())
    }

    async fn find_all(&self) -> Result<Vec<User>, DbError> {
        Ok(self.users.lock().unwrap().clone())
    }

    async fn insert(&self, entity: User) -> Result<User, DbError> {
        self.users.lock().unwrap().push(entity.clone());
        Ok(entity)
    }

    async fn update(&self, entity: User) -> Result<(), DbError> {
        let mut users = self.users.lock().unwrap();
        if let Some(slot) = users.iter_mut().find(|u| u.id == entity.id) {
            *slot = entity;
            Ok(())
        } else {
            Err(DbError::NotFoundError(entity.id))
        }
    }

    async fn delete(&self, id: &str) -> Result<(), DbError> {
        self.users.lock().unwrap().retain(|u| u.id != id);
        Ok(())
    }
}

fn build_router() -> Router<()> {
    let repo: Arc<dyn Repository<User>> = Arc::new(InMemoryRepo::default());
    let service = GrpcService::new("users", repo, JsonCodec);
    let handler = GrpcHandler::new(service);
    Router::new().post("/grpc", handler)
}

fn context(body: Vec<u8>, grpc_method: &str) -> Context<()> {
    let mut req = Request::new(HttpMethod::Post, "/grpc".to_string(), body);
    req.headers.insert("grpc-method", grpc_method.to_string());
    Context::new(Arc::new(()), req)
}

#[tokio::main]
async fn main() {
    println!("=== gerax-grpc CRUD example ===\n");

    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .field("email", RustType::String)
        .build();
    println!("proto package: {:?}", schema.package_name());
    println!(
        "proto services: {:?}\n",
        schema.services().collect::<Vec<_>>()
    );

    let router = build_router();

    let alice = User {
        id: "1".to_string(),
        name: "Alice".to_string(),
    };

    // create_users
    let body = serde_json::to_vec(&alice).unwrap();
    let res = router
        .handle(context(body, "create_users"))
        .await
        .expect("create ok");
    println!("create_users -> {} {:?}", res.status, res.body);
    let created: User = serde_json::from_slice(&res.body).unwrap();
    println!("created: {:?}\n", created);

    // read_users
    let res = router
        .handle(context("1".to_string().into_bytes(), "read_users"))
        .await
        .expect("read ok");
    println!("read_users(1) -> {} {:?}", res.status, res.body);
    let found: User = serde_json::from_slice(&res.body).unwrap();
    println!("found: {:?}\n", found);

    // update_users
    let mut updated = found.clone();
    updated.name = "Alice Cooper".to_string();
    let body = serde_json::to_vec(&updated).unwrap();
    let res = router
        .handle(context(body, "update_users"))
        .await
        .expect("update ok");
    println!("update_users -> {}\n", res.status);

    // list_users
    let res = router
        .handle(context(vec![], "list_users"))
        .await
        .expect("list ok");
    println!("list_users -> {} {:?}", res.status, res.body);
    let list: Vec<User> = serde_json::from_slice(&res.body).unwrap();
    println!("count: {}\n", list.len());

    // delete_users
    let res = router
        .handle(context("1".to_string().into_bytes(), "delete_users"))
        .await
        .expect("delete ok");
    println!("delete_users -> {}\n", res.status);

    // list after delete
    let res = router
        .handle(context(vec![], "list_users"))
        .await
        .expect("list ok");
    println!("list_users after delete -> {} {:?}", res.status, res.body);
    let list: Vec<User> = serde_json::from_slice(&res.body).unwrap();
    println!("count: {}\n", list.len());

    // GrpcClient configuration example
    let _client = GrpcClient::<User>::new("http://127.0.0.1:50051", JsonCodec);
    println!("GrpcClient configured for http://127.0.0.1:50051");
    println!(
        "GrpcServer::from_config example addr: {}",
        GrpcConfig::default().socket_addr()
    );
}
