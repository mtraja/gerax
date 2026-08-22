use std::{path::PathBuf, sync::Arc};

use serde::de::DeserializeOwned;

use crate::{
    error::ConfigResult,
    loader::ConfigLoader,
    source::{
        ConfigSource, env::EnvSource, json::JsonSource, memory::MemorySource, toml::TomlSource,
        yaml::YamlSource,
    },
};

/// Constroe configuracao
#[derive(Debug, Default)]
pub struct ConfigBuilder {
    sources: Vec<Arc<dyn ConfigSource>>,
}

impl ConfigBuilder {
    /// Cria um builder vazio.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Adiciona uma fonte genérica.
    pub fn source<S>(mut self, source: S) -> Self
    where
        S: ConfigSource + 'static,
    {
        self.sources.push(Arc::new(source));

        self
    }

    /// Carrega arquivo TOML.
    pub fn toml(self, path: impl Into<PathBuf>) -> Self {
        self.source(TomlSource::new(path))
    }

    /// Carrega arquivo YAML.
    pub fn yaml(self, path: impl Into<PathBuf>) -> Self {
        self.source(YamlSource::new(path))
    }

    /// Carrega arquivo JSON.
    pub fn json(self, path: impl Into<PathBuf>) -> Self {
        self.source(JsonSource::new(path))
    }

    /// Carrega arquivo .env.
    pub fn env(self) -> Self {
        self.source(EnvSource::dotenv())
    }

    /// Carrega variáveis do sistema.
    pub fn system_env(self) -> Self {
        self.source(EnvSource::system())
    }

    /// Adiciona configuração em memória.
    pub fn memory(self, source: MemorySource) -> Self {
        self.source(source)
    }

    /// Constrói o loader.
    pub fn loader(self) -> ConfigLoader {
        let mut loader = ConfigLoader::new();

        for source in self.sources {
            loader = loader.with_boxed_source(source);
        }

        loader
    }

    /// Carrega a configuração final.
    pub fn build<T>(self) -> ConfigResult<T>
    where
        T: DeserializeOwned,
    {
        self.loader().load::<T>()
    }
}
