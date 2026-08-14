//! Testes de integração do `GrpcHandler` com `gerax-http`.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use gerax_codec::JsonCodec;
use gerax_core::Entity;
use gerax_db::{DbError, Repository};
use gerax_grpc::{GrpcHandler, GrpcService};
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
    users: Mutex<Vec<User>>,
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

fn build_router() -> Router<()> {
    let repo: Arc<dyn Repository<User>> = Arc::new(InMemoryRepo::default());
    let service = GrpcService::new("users", repo, JsonCodec);

    let handler = GrpcHandler::new(service);
    Router::new().post("/grpc", handler)
}

fn context(method: HttpMethod, body: Vec<u8>, grpc_method: &str) -> Context<()> {
    let mut request = Request::new(method, "/grpc".to_string(), body);
    request
        .headers
        .insert("grpc-method", grpc_method.to_string());
    Context::new(Arc::new(()), request)
}

#[tokio::test]
async fn grpc_handler_insert_and_read() {
    let router = build_router();

    let user = User {
        id: "1".to_string(),
        name: "Alice".to_string(),
    };
    let body = serde_json::to_vec(&user).unwrap();

    // create_users
    let response = router
        .handle(context(HttpMethod::Post, body.clone(), "create_users"))
        .await
        .expect("handler ok");
    assert_eq!(response.status, 200);
    let created: User = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(created, user);

    // read_users (by id encoded as utf8 string payload)
    let response = router
        .handle(context(
            HttpMethod::Post,
            "1".to_string().into_bytes(),
            "read_users",
        ))
        .await
        .expect("handler ok");
    assert_eq!(response.status, 200);
    let found: User = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(found, user);

    // read_users with unknown id -> 404
    let response = router
        .handle(context(
            HttpMethod::Post,
            "missing".to_string().into_bytes(),
            "read_users",
        ))
        .await
        .expect("handler ok");
    assert_eq!(response.status, 404);
}

#[tokio::test]
async fn grpc_handler_missing_method_header() {
    let router = build_router();
    let ctx = {
        let request = Request::new(HttpMethod::Post, "/grpc".to_string(), vec![]);
        Context::new(Arc::new(()), request)
    };
    let result = router.handle(ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn grpc_handler_list() {
    let router = build_router();
    let user = User {
        id: "1".to_string(),
        name: "Alice".to_string(),
    };
    router
        .handle(context(
            HttpMethod::Post,
            serde_json::to_vec(&user).unwrap(),
            "create_users",
        ))
        .await
        .unwrap();

    let response = router
        .handle(context(HttpMethod::Post, vec![], "list_users"))
        .await
        .unwrap();
    assert_eq!(response.status, 200);
    let list: Vec<User> = serde_json::from_slice(&response.body).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0], user);
}
