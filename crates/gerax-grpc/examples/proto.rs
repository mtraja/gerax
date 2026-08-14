//! Exemplo: uso de `ProtoBuilder` para gerar um schema Protobuf
//! a partir de uma entidade `gerax-core`.
//!
//! Execute com:
//!
//! ```bash
//! cargo run --example proto -p gerax-grpc
//! ```

use gerax_core::Entity;
use gerax_grpc::proto::{ProtoBuilder, RustType};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct User {
    id: Option<String>,
    name: String,
    email: String,
}

impl Entity for User {
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

fn main() {
    let schema = ProtoBuilder::new()
        .entity::<User>()
        .field("name", RustType::String)
        .field("email", RustType::String)
        .build();

    println!("package: {:?}", schema.package_name());
    println!("services: {:?}", schema.services().collect::<Vec<_>>());
    println!("\n--- schema ---\n{}", schema.content);
}
