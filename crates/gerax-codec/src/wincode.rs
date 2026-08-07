use super::codec::{Codec, CodecError};

pub struct WincodeCodec;

impl<T> Codec<T> for WincodeCodec
where
    T: wincode::Serialize<Src = T> + wincode::DeserializeOwned<Dst = T>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        Ok(wincode::serialize(value)?)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        Ok(wincode::deserialize(bytes)?)
    }
}