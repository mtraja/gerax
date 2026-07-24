use std::fmt::Debug;

use crate::{document::ConfigDocument, error::ConfigResult};

pub mod env;
pub mod json;
pub mod memory;
pub mod toml;
pub mod yaml;

/// Origem de uma configuração.
///
/// Uma aplicação pode possuir várias fontes:
///
/// - memória
/// - .env
/// - TOML
/// - YAML
/// - JSON
///
/// Todas são convertidas para ConfigDocument.
pub trait ConfigSource: Debug + Send + Sync {
    /// Nome da fonte.
    fn name(&self) -> &'static str;

    /// Carrega a configuração.
    fn load(&self) -> ConfigResult<ConfigDocument>;
}
