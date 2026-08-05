use std::fmt::Display;

/// Identificador do formato desejado
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializerFormat {
    Json,
    Bincode, // Formato binário
}

/// Erro genérico da camada de serialização
#[derive(Debug)]
pub struct CodecError(pub String);

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CodecError: {}", self.0)
    }
}

impl std::error::Error for CodecError {}

/// Trait que define o contrato do Codec
pub trait Codec<T> {
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError>;
    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError>;
}