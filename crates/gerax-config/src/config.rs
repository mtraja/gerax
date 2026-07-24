use crate::builder::ConfigBuilder;


/// Entrada principal do sistema de configuração.
///
/// Exemplo:
///
/// ```rust
/// let config = Config::builder()
///     .env()
///     .toml("config.toml")
///     .build::<AppConfig>()?;
/// ```
pub struct Config;


impl Config {


    /// Cria um novo builder.
    pub fn builder() -> ConfigBuilder {

        ConfigBuilder::new()
    }
}
