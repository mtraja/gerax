//! Testes de integração: `GrpcServer` + `GrpcClient` com repositório em memória.

use std::sync::Arc;

use async_trait::async_trait;
use gerax_codec::JsonCodec;
use gerax_core::Entity;
use gerax_db::{DbError, Repository};
use gerax_grpc::{GrpcClient, GrpcServer, GrpcService};

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
        let mut users = self.users.lock().unwrap();
        users.retain(|u| u.id != id);
        Ok(())
    }
}

fn service() -> GrpcService<User, JsonCodec> {
    let repo: Arc<dyn Repository<User>> = Arc::new(InMemoryRepo::default());
    GrpcService::new("users", repo, JsonCodec)
}

#[tokio::test]
async fn grpc_server_serve_rejects_invalid_addr() {
    let server = GrpcServer::new(service(), "not-an-address");
    let result = server.serve().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn grpc_server_starts_and_serves() {
    let server = GrpcServer::new(service(), "127.0.0.1:50052");
    let handle = tokio::spawn(async move { server.serve().await });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    handle.abort();
}

#[tokio::test]
async fn grpc_client_connect_fails_without_server() {
    let client = GrpcClient::<User>::new("127.0.0.1:1", JsonCodec);
    let result = client.connect().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn grpc_client_roundtrip_crud() {
    let server = GrpcServer::new(service(), "127.0.0.1:50053");
    let server_handle = tokio::spawn(async move { server.serve().await });

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    let client = GrpcClient::<User>::new("http://127.0.0.1:50053", JsonCodec);
    client.connect().await.expect("connect ok");

    let alice = User {
        id: "1".to_string(),
        name: "Alice".to_string(),
    };

    let created = client.insert(alice.clone()).await.expect("insert ok");
    assert_eq!(created, alice);

    let found = client.find_by_id("1").await.expect("find_by_id ok");
    assert_eq!(found, Some(alice.clone()));

    let list = client.find_all().await.expect("find_all ok");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0], alice);

    let mut updated = alice.clone();
    updated.name = "Alice Cooper".to_string();
    client.update(updated.clone()).await.expect("update ok");

    let found = client
        .find_by_id("1")
        .await
        .expect("find_by_id after update ok");
    assert_eq!(found, Some(updated));

    client.delete("1").await.expect("delete ok");

    let found = client
        .find_by_id("1")
        .await
        .expect("find_by_id after delete ok");
    assert!(found.is_none());

    server_handle.abort();
}
