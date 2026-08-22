use serde::Serialize;
//use serde_json::Value;

use crate::{
    document::ConfigDocument,
    error::{ConfigError, ConfigResult},
    source::ConfigSource,
};

/// Representa uma fonte em Memoria
#[derive(Debug, Clone)]
pub struct MemorySource {
    document: ConfigDocument,
}

impl MemorySource {
    /// Cria uma fonte vazia.
    pub fn new() -> Self {
        Self {
            document: ConfigDocument::new(),
        }
    }

    /// Cria a partir de um documento existente.
    pub fn from_document(document: ConfigDocument) -> Self {
        Self { document }
    }

    /// Cria a partir de qualquer estrutura serializável.
    ///
    /// Exemplo:
    ///
    /// ```ignore
    /// MemorySource::from_value(
    ///     &MyConfig { ... }
    /// )
    /// ```
    pub fn from_value<T>(value: &T) -> ConfigResult<Self>
    where
        T: Serialize,
    {
        let value =
            serde_json::to_value(value).map_err(|e| ConfigError::Deserialize(e.to_string()))?;

        let document = ConfigDocument::from_value(value)?;

        Ok(Self { document })
    }

    /// Insere uma configuração simples.
    ///
    /// Ex:
    ///
    /// server.port = 8080
    ///
    pub fn insert(mut self, key: impl Into<String>, value: impl Serialize) -> ConfigResult<Self> {
        let mut root = self.document.into_value();

        let object = root
            .as_object_mut()
            .ok_or_else(|| ConfigError::InvalidConfiguration("root must be object".into()))?;

        object.insert(
            key.into(),
            serde_json::to_value(value).map_err(|e| ConfigError::Deserialize(e.to_string()))?,
        );

        self.document = ConfigDocument::from_value(root)?;

        Ok(self)
    }
}

impl Default for MemorySource {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for MemorySource {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn load(&self) -> ConfigResult<ConfigDocument> {
        Ok(self.document.clone())
    }
}
