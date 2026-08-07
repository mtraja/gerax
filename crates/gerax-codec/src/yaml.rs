use serde::{Deserialize, Serialize};
use crate::codec::Codec;
pub struct YamlCodec;

impl<T> Codec<T> for YamlCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, crate::codec::CodecError> {
        Ok(serde_yml::to_string(value)?.into_bytes())
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, crate::codec::CodecError> {
        let yaml_str = std::str::from_utf8(bytes)?;
        Ok(serde_yml::from_str(yaml_str)?)
    }
}