//! Testes de serialização/deserialização de entidades via codec.
//!
//! Como o `gerax-grpc` utiliza `JsonCodec` como codec padrão e os stubs
//! Protobuf ainda não foram gerados por `tonic-build`, estes testes focam
//! no round-trip JSON. Quando o schema Protobuf estiver integrado, a
//! mesma estrutura de teste pode ser reutilizada com `ProtobufCodec`.

use gerax_codec::{Codec, JsonCodec};
use gerax_core::Entity;

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

#[test]
fn json_codec_roundtrip_entity() {
    let codec = JsonCodec;
    let user = User {
        id: "1".to_string(),
        name: "Alice".to_string(),
    };

    let bytes = codec.serialize(&user).expect("serialize entity");
    let decoded: User = codec.deserialize(&bytes).expect("deserialize entity");

    assert_eq!(decoded, user);
}

#[test]
fn json_codec_roundtrip_vec_entity() {
    let codec = JsonCodec;
    let users = vec![
        User {
            id: "1".to_string(),
            name: "Alice".to_string(),
        },
        User {
            id: "2".to_string(),
            name: "Bob".to_string(),
        },
    ];

    let bytes = codec.serialize(&users).expect("serialize vec");
    let decoded: Vec<User> = codec.deserialize(&bytes).expect("deserialize vec");

    assert_eq!(decoded, users);
}
