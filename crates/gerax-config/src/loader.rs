use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::{document::ConfigDocument, error::ConfigResult, source::ConfigSource};

/// Carrega fonte de formato de arquivo
#[derive(Debug, Default)]
pub struct ConfigLoader {
    sources: Vec<Arc<dyn ConfigSource>>,
}

impl ConfigLoader {
    /// Cria um loader vazio.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Adiciona uma fonte.
    pub fn with_source<S>(mut self, source: S) -> Self
    where
        S: ConfigSource + 'static,
    {
        self.sources.push(Arc::new(source));

        self
    }

    /// Adiciona uma fonte usando referência.
    pub fn with_boxed_source(mut self, source: Arc<dyn ConfigSource>) -> Self {
        self.sources.push(source);

        self
    }

    /// Carrega todas as fontes.
    ///
    /// A ordem importa:
    ///
    /// última fonte sobrescreve anteriores.
    pub fn load_document(&self) -> ConfigResult<ConfigDocument> {
        let mut document = ConfigDocument::new();

        for source in &self.sources {
            let loaded = source.load()?;

            document.merge(loaded);
        }

        Ok(document)
    }

    /// Carrega e desserializa.
    pub fn load<T>(&self) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        let document = self.load_document()?;

        document.deserialize()
    }
}
