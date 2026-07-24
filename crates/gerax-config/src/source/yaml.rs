use std::path::{Path, PathBuf};

use crate::{
    document::ConfigDocument,
    error::ConfigResult,
    internal::{file, parser},
    source::ConfigSource,
};

/// Representa uma fonte Yaml
#[derive(Debug, Clone)]
pub struct YamlSource {
    path: PathBuf,
}

impl YamlSource {
    /// Cria uma fonte YAML.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Retorna o caminho do arquivo.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ConfigSource for YamlSource {
    fn name(&self) -> &'static str {
        "yaml"
    }

    fn load(&self) -> ConfigResult<ConfigDocument> {
        let content = file::read(&self.path)?;

        parser::yaml(&content)
    }
}
