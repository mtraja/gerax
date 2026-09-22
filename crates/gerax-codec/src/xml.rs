use crate::codec::Codec;
use serde::{Deserialize, Serialize};
pub struct XmlCodec;

impl<T> Codec<T> for XmlCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, crate::codec::CodecError> {
        Ok(quick_xml::se::to_string(value)?.into_bytes())
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, crate::codec::CodecError> {
        let xml_str = std::str::from_utf8(bytes)?;
        Ok(quick_xml::de::from_str(xml_str)?)
    }
}