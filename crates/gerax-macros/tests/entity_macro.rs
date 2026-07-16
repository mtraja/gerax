use gerax_core::Entity;
use gerax_macros::Entity as EntityMacro;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EntityMacro)]
struct User {
    id: Option<String>,
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EntityMacro)]
struct Person {
    id: Option<String>,
    full_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EntityMacro)]
#[entity(collection_name = "people")]
struct CustomPerson {
    id: Option<String>,
    full_name: String,
}

#[test]
fn entity_macro_generates_correct_impl() {
    assert_eq!(User::collection_name(), "users");
    assert_eq!(Person::collection_name(), "persons");
    assert_eq!(CustomPerson::collection_name(), "people");

    let mut user = User {
        id: None,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    user.set_id("123".to_string());
    assert_eq!(user.id, Some("123".to_string()));
    assert_eq!(user.id(), Some("123".to_string()));
}
