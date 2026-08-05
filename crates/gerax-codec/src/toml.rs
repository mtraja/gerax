use serde::{Deserialize, Serialize};
use crate::codec::{Codec, CodecError};
pub struct TomlCodec;

impl<T> Codec<T> for TomlCodec
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError> {
        // toml::to_string converte a struct para uma String no formato TOML
        let toml_string = toml::to_string(value)
            .map_err(|e| CodecError(format!("Erro ao serializar TOML: {}", e)))?;
        
        Ok(toml_string.into_bytes())
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError> {
        // Valida se os bytes recebidos sao UTF-8 validos
        let toml_str = std::str::from_utf8(bytes)
            .map_err(|e| CodecError(format!("Bytes UTF-8 invalidos para TOML: {}", e)))?;

        // Desserializa a string TOML para a struct T
        toml::from_str(toml_str)
            .map_err(|e| CodecError(format!("Erro ao desserializar TOML: {}", e)))
    }
}