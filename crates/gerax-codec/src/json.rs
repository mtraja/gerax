use serde::{Deserialize, Serialize};
use crate::codec::Codec;
pub struct JsonCodec;

impl<T> Codec<T> for JsonCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, crate::codec::CodecError> {
        Ok(serde_json::to_vec(value)?)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, crate::codec::CodecError> {
        Ok(serde_json::from_slice(bytes)?)
    }
}