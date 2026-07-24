use std::path::{Path, PathBuf};

use crate::{
    document::ConfigDocument,
    error::ConfigResult,
    internal::{file, parser},
    source::ConfigSource,
};

/// Representa uma fonte Json
#[derive(Debug, Clone)]
pub struct JsonSource {
    path: PathBuf,
}

impl JsonSource {
    /// Cria uma fonte JSON.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Retorna o caminho do arquivo.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ConfigSource for JsonSource {
    fn name(&self) -> &'static str {
        "json"
    }

    fn load(&self) -> ConfigResult<ConfigDocument> {
        let content = file::read(&self.path)?;

        parser::json(&content)
    }
}
