use serde::{Deserialize, Serialize};
use crate::codec::{Codec, CodecError};
pub struct JsonCodec;

impl<T> Codec<T> for JsonCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(value)
            .map_err(|e| CodecError(format!("Erro ao serializar JSON: {}", e)))
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        serde_json::from_slice(bytes)
            .map_err(|e| CodecError(format!("Erro ao desserializar JSON: {}", e)))
    }
}