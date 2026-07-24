use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{ConfigError, ConfigResult};

/// Documento interno de configuração.
///
/// Todas as fontes de configuração são convertidas
/// para este formato.
#[derive(Debug, Clone)]
pub struct ConfigDocument {
    root: Value,
}

impl ConfigDocument {
    /// Cria um documento vazio.
    pub fn new() -> Self {
        Self {
            root: Value::Object(Default::default()),
        }
    }

    /// Cria um documento a partir de um Value.
    pub(crate) fn from_value(value: Value) -> ConfigResult<Self> {
        match value {
            Value::Object(_) => Ok(Self { root: value }),

            _ => Err(ConfigError::InvalidConfiguration(
                "configuration root must be an object".into(),
            )),
        }
    }

    /// Retorna o valor interno.
    pub fn value(&self) -> &Value {
        &self.root
    }

    /// Consome o documento.
    pub fn into_value(self) -> Value {
        self.root
    }

    /// Faz merge recursivo.
    pub(crate) fn merge(
        &mut self,
        other: ConfigDocument,
    ) {
        crate::internal::merge::merge(
            &mut self.root,
            other.root,
        );
    }

    /// Desserializa para uma estrutura Rust.
    pub fn deserialize<T>(&self) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(self.root.clone())
            .map_err(|e| {
                ConfigError::Deserialize(
                    e.to_string(),
                )
            })
    }
}

impl Default for ConfigDocument {
    fn default() -> Self {
        Self::new()
    }
}
