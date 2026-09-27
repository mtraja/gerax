//! Codec MessagePack (MsgPack) via [`rmp_serde`].
//!
//! MessagePack é um formato binário compacto, mais leve que JSON e mais
//! rápido de serializar, muito usado em filas de mensagens, caches e
//! comunicação entre microsserviços.

use super::codec::{Codec, CodecError};
use serde::{Deserialize, Serialize};

/// Codec MessagePack via `rmp-serde`.
///
/// A serialização usa a representação **compacta** (structs como arrays),
/// que é a mais eficiente em tamanho. Use [`MessagePackCodec::serialize_named`]
/// quando o consumidor esperar structs como mapas nomeados.
pub struct MessagePackCodec;

impl MessagePackCodec {
    /// Serializa usando structs como mapas nomeados, em vez de arrays.
    ///
    /// Útil para interoperabilidade com consumidores que não assumem a ordem
    /// dos campos (ex.: serviços em outras linguagens).
    pub fn serialize_named<T>(value: &T) -> Result<Vec<u8>, CodecError>
    where
        T: Serialize,
    {
        Ok(rmp_serde::to_vec_named(value)?)
    }
}

impl<T> Codec<T> for MessagePackCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        Ok(rmp_serde::to_vec(value)?)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use super::MessagePackCodec;
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
            tags: vec!["codec".to_string(), "msgpack".to_string()],
            meta: Some(HashMap::from_iter([("retries".to_string(), 3)])),
        }
    }

    #[test]
    fn round_trips_message() {
        let codec = MessagePackCodec;
        let original = sample();

        let bytes = codec.serialize(&original).expect("serializar");
        let decoded: Message = codec.deserialize(&bytes).expect("deserializar");

        assert_eq!(original, decoded);
    }

    #[test]
    fn round_trips_named() {
        let original = sample();

        let bytes = MessagePackCodec::serialize_named(&original).expect("serializar named");
        let decoded: Message = MessagePackCodec
            .deserialize(&bytes)
            .expect("deserializar named");

        assert_eq!(original, decoded);
    }

    #[test]
    fn compact_is_smaller_than_named() {
        let original = sample();

        let compact = MessagePackCodec.serialize(&original).expect("serializar");
        let named = MessagePackCodec::serialize_named(&original).expect("serializar named");

        assert!(compact.len() < named.len());
    }

    #[test]
    fn round_trips_optional_fields() {
        let original = Message {
            meta: None,
            ..sample()
        };

        let bytes = MessagePackCodec.serialize(&original).expect("serializar");
        let decoded: Message = MessagePackCodec.deserialize(&bytes).expect("deserializar");

        assert_eq!(original, decoded);
    }

    #[test]
    fn fails_to_deserialize_invalid_bytes() {
        let result: Result<Message, _> = MessagePackCodec.deserialize(&[0xc1, 0xc1, 0xc1]);

        assert!(result.is_err());
    }

    #[test]
    fn fails_on_type_mismatch() {
        let bytes = MessagePackCodec.serialize(&sample()).expect("serializar");

        let result: Result<Vec<i32>, _> = MessagePackCodec.deserialize(&bytes);

        assert!(result.is_err());
    }
}
