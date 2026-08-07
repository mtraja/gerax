use prost::Message;
use super::codec::{Codec, CodecError};

/// Codec Protobuf via `prost`.
///
/// O bound `Default` é necessário porque `prost::Message::decode` (v0.14)
/// exige `Self: Default` internamente.
pub struct ProtobufCodec;

impl<T> Codec<T> for ProtobufCodec
where
    T: Message + Default,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        let mut buf = Vec::with_capacity(value.encoded_len());
        value.encode(&mut buf)?;
        Ok(buf)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        Ok(T::decode(bytes)?)
    }
}