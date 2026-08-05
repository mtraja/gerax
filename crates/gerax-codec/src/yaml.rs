use serde::{Deserialize, Serialize};
use crate::codec::{Codec, CodecError};
pub struct YamlCodec;

impl<T> Codec<T> for YamlCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        // Converte a struct para String YAML e depois extrai o vetor de bytes (Vec<u8>)
        serde_yml::to_string(value)
            .map_err(|e| CodecError(format!("Erro ao serializar YAML: {}", e)))
            .map(|s| s.into_bytes())
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        // Converte a slice de bytes em &str UTF-8 e desserializa
        let yaml_str = std::str::from_utf8(bytes)
            .map_err(|e| CodecError(format!("Bytes UTF-8 inválidos para YAML: {}", e)))?;

        serde_yml::from_str(yaml_str)
            .map_err(|e| CodecError(format!("Erro ao desserializar YAML: {}", e)))
    }
}