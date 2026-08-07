use serde::{Deserialize, Serialize};
use crate::codec::Codec;
pub struct TomlCodec;

impl<T> Codec<T> for TomlCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, crate::codec::CodecError> {
        Ok(toml::to_string(value)?.into_bytes())
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, crate::codec::CodecError> {
        let toml_str = std::str::from_utf8(bytes)?;
        Ok(toml::from_str(toml_str)?)
    }
}