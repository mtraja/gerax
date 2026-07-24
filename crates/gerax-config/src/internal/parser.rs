use serde_json::Value;

use crate::{
    document::ConfigDocument,
    error::{ConfigError, ConfigResult},
    internal::convert,
};


/// Faz parse de TOML.
#[cfg(feature = "toml")]
pub(crate) fn toml(
    content: &str,
) -> ConfigResult<ConfigDocument> {

    let value: toml::Value =
        toml::from_str(content)
            .map_err(|e| {

                ConfigError::Toml {
                    path: "<memory>".into(),
                    source: e,
                }

            })?;


    let value =
        convert::from_toml(value)?;


    let value =
        convert::ensure_object(value)?;


    ConfigDocument::from_value(value)
}



/// Faz parse de YAML.
#[cfg(feature = "yaml")]
pub(crate) fn yaml(
    content: &str,
) -> ConfigResult<ConfigDocument> {

    let value: serde_yaml::Value =
        serde_yaml::from_str(content)
            .map_err(|e| {

                ConfigError::Yaml {
                    path: "<memory>".into(),
                    source: e,
                }

            })?;


    let value =
        convert::from_yaml(value)?;


    let value =
        convert::ensure_object(value)?;


    ConfigDocument::from_value(value)
}



/// Faz parse de JSON.
#[cfg(feature = "json")]
pub(crate) fn json(
    content: &str,
) -> ConfigResult<ConfigDocument> {

    let value: Value =
        serde_json::from_str(content)
            .map_err(|e| {

                ConfigError::Json {
                    path: "<memory>".into(),
                    source: e,
                }

            })?;


    let value =
        convert::ensure_object(value)?;


    ConfigDocument::from_value(value)
}