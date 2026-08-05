//use bytes::BytesMut;
use prost::Message;
use super::codec::{Codec, CodecError};

pub struct ProtobufCodec;

impl<T> Codec<T> for ProtobufCodec
where
    T: Message + Default,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        let mut buf = Vec::with_capacity(value.encoded_len());
        value
            .encode(&mut buf)
            .map_err(|e| CodecError(format!("Erro ao codificar Protobuf: {}", e)))?;
        Ok(buf)
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        T::decode(bytes)
            .map_err(|e| CodecError(format!("Erro ao decodificar Protobuf: {}", e)))
    }
}