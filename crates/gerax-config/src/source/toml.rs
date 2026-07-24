use std::path::{Path, PathBuf};

use crate::{
    document::ConfigDocument,
    error::ConfigResult,
    internal::{file, parser},
    source::ConfigSource,
};

/// Representa uma fonte Toml
#[derive(Debug, Clone)]
pub struct TomlSource {
    path: PathBuf,
}

impl TomlSource {
    /// Cria uma fonte TOML.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Retorna o caminho do arquivo.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ConfigSource for TomlSource {
    fn name(&self) -> &'static str {
        "toml"
    }

    fn load(&self) -> ConfigResult<ConfigDocument> {
        let content = file::read(&self.path)?;

        parser::toml(&content)
    }
}
