//! Codec TOON (Token-Oriented Object Notation) via [`serde_toon_format`].
//!
//! TOON é um formato textual alternativo ao JSON, pensado para prompts de LLMs:
//! usa indentação em vez de chaves e arrays tabulares para objetos uniformes,
//! reduzindo o número de tokens sem perder o modelo de dados do JSON.

use super::codec::{Codec, CodecError};
use serde::{Deserialize, Serialize};

/// Codec TOON via `serde_toon_format`.
///
/// A serialização gera texto TOON (UTF-8). A desserialização aceita tanto TOON
/// quanto JSON, já que o formato compartilha o mesmo modelo de dados.
pub struct ToonCodec;

impl<T> Codec<T> for ToonCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        Ok(serde_toon::to_vec(value)?)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        Ok(serde_toon::from_slice(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use super::ToonCodec;
    use crate::codec::Codec;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Message {
        id: u64,
        name: String,
        tags: Vec<String>,
        meta: Option<HashMap<String, i32>>,
    }

    fn sample() -> Message {
        Message {
            id: 7,
            name: "gerax".to_string(),
            tags: vec!["codec".to_string(), "toon".to_string()],
            meta: Some(HashMap::from_iter([("retries".to_string(), 3)])),
        }
    }

    #[test]
    fn serializes_to_readable_toon() {
        let bytes = ToonCodec.serialize(&sample()).expect("serializar");
        let toon = std::str::from_utf8(&bytes).expect("utf-8");

        assert!(toon.contains("id: 7"));
        assert!(toon.contains("name: gerax"));
    }

    #[test]
    fn round_trips_message() {
        let codec = ToonCodec;
        let original = sample();

        let bytes = codec.serialize(&original).expect("serializar");
        let decoded: Message = codec.deserialize(&bytes).expect("deserializar");

        assert_eq!(original, decoded);
    }

    #[test]
    fn round_trips_optional_fields() {
        let original = Message {
            meta: None,
            ..sample()
        };

        let bytes = ToonCodec.serialize(&original).expect("serializar");
        let decoded: Message = ToonCodec.deserialize(&bytes).expect("deserializar");

        assert_eq!(original, decoded);
    }

    #[test]
    fn round_trips_uniform_objects_as_tabular_array() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Row {
            id: u32,
            name: String,
        }

        let original = vec![
            Row {
                id: 1,
                name: "alpha".to_string(),
            },
            Row {
                id: 2,
                name: "beta".to_string(),
            },
        ];

        let bytes = ToonCodec.serialize(&original).expect("serializar");
        let toon = std::str::from_utf8(&bytes).expect("utf-8");
        assert!(toon.starts_with("[2]{"));

        let decoded: Vec<Row> = ToonCodec.deserialize(&bytes).expect("deserializar");
        assert_eq!(original, decoded);
    }

    #[test]
    fn fails_to_deserialize_invalid_bytes() {
        let result: Result<Message, _> = ToonCodec.deserialize(&[0xff, 0xfe, 0xfd]);

        assert!(result.is_err());
    }

    #[test]
    fn fails_on_type_mismatch() {
        let bytes = ToonCodec.serialize(&sample()).expect("serializar");

        let result: Result<Vec<i32>, _> = ToonCodec.deserialize(&bytes);

        assert!(result.is_err());
    }
}
