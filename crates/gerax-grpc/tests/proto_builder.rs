use gerax_grpc::proto::{ProtoBuilder, RustType};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct User {
    id: Option<String>,
    name: String,
    email: String,
}

impl gerax_core::Entity for User {
    fn collection_name() -> &'static str {
        "users"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[test]
fn proto_builder_generates_valid_schema() {
    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .field("email", RustType::String)
        .build();

    assert!(schema.content.contains("syntax = \"proto3\";"));
    assert!(schema.content.contains("package gerax;"));
    assert!(schema.content.contains("message Users {"));
    assert!(schema.content.contains("string name = 1;"));
    assert!(schema.content.contains("string email = 2;"));
}

#[test]
fn proto_builder_generates_crud_service() {
    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .build();

    assert!(schema.content.contains("service EntityService {"));
    assert!(schema.content.contains("rpc CreateUsers"));
    assert!(schema.content.contains("rpc ReadUsers"));
    assert!(schema.content.contains("rpc UpdateUsers"));
    assert!(schema.content.contains("rpc DeleteUsers"));
    assert!(schema.content.contains("rpc ListUsers"));
}

#[test]
fn proto_builder_generates_request_response_messages() {
    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .build();

    assert!(schema.content.contains("message CreateUsersRequest {"));
    assert!(schema.content.contains("message CreateUsersResponse {"));
    assert!(schema.content.contains("message ReadUsersRequest {"));
    assert!(schema.content.contains("string id = 1;"));
    assert!(schema.content.contains("message ReadUsersResponse {"));
    assert!(schema.content.contains("message UpdateUsersRequest {"));
    assert!(schema.content.contains("message UpdateUsersResponse {"));
    assert!(schema.content.contains("message DeleteUsersRequest {"));
    assert!(schema.content.contains("message DeleteUsersResponse {"));
    assert!(schema.content.contains("message ListUsersRequest {"));
    assert!(schema.content.contains("message ListUsersResponse {"));
}

#[test]
fn proto_schema_package_name() {
    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .build();

    assert_eq!(schema.package_name(), Some("gerax"));
}

#[test]
fn proto_schema_services() {
    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .build();

    let services: Vec<_> = schema.services().collect();
    assert_eq!(services, vec!["EntityService"]);
}

#[test]
fn proto_builder_multiple_entities() {
    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .entity::<Post>()
        .field("title", RustType::String)
        .field("content", RustType::String)
        .build();

    assert!(schema.content.contains("message Users {"));
    assert!(schema.content.contains("message Posts {"));
    assert!(schema.content.contains("rpc CreatePosts"));
    assert!(schema.content.contains("rpc ReadUsers"));
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Post {
    id: Option<String>,
    title: String,
    content: String,
}

impl gerax_core::Entity for Post {
    fn collection_name() -> &'static str {
        "posts"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}
