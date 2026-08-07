/// Erro genérico da camada de serialização
#[derive(Debug)]
#[must_use]
pub struct CodecError(pub String);

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CodecError: {}", self.0)
    }
}

impl std::error::Error for CodecError {}

impl From<serde_json::Error> for CodecError {
    fn from(e: serde_json::Error) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<serde_yml::Error> for CodecError {
    fn from(e: serde_yml::Error) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<toml::de::Error> for CodecError {
    fn from(e: toml::de::Error) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<toml::ser::Error> for CodecError {
    fn from(e: toml::ser::Error) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<prost::DecodeError> for CodecError {
    fn from(e: prost::DecodeError) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<capnp::Error> for CodecError {
    fn from(e: capnp::Error) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<prost::EncodeError> for CodecError {
    fn from(e: prost::EncodeError) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<std::str::Utf8Error> for CodecError {
    fn from(e: std::str::Utf8Error) -> Self {
        CodecError(format!("{}", e))
    }
}

impl From<wincode::WriteError> for CodecError {
    fn from(e: wincode::WriteError) -> Self {
        CodecError(e.to_string())
    }
}

impl From<wincode::ReadError> for CodecError {
    fn from(e: wincode::ReadError) -> Self {
        CodecError(e.to_string())
    }
}

/// Trait que define o contrato do Codec
pub trait Codec<T> {
    fn serialize(&self, value: &T) -> Result<Vec<u8>, CodecError>;
    fn deserialize(&self, bytes: &[u8]) -> Result<T, CodecError>;
}
