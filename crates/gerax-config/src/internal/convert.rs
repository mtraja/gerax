use serde_json::Value;

use crate::{
    error::{ConfigError, ConfigResult},
};


/// Converte uma estrutura TOML para JSON Value.
///
/// O crate `toml` possui uma estrutura própria.
/// Este método normaliza para o formato interno.
#[cfg(feature = "toml")]
pub(crate) fn from_toml(
    value: toml::Value,
) -> ConfigResult<Value> {

    serde_json::to_value(value)
        .map_err(|e| {

            ConfigError::Deserialize(
                e.to_string(),
            )

        })
}


/// Converte uma estrutura YAML para JSON Value.
#[cfg(feature = "yaml")]
pub(crate) fn from_yaml(
    value: serde_yaml::Value,
) -> ConfigResult<Value> {

    serde_json::to_value(value)
        .map_err(|e| {

            ConfigError::Deserialize(
                e.to_string(),
            )

        })
}


/// Valida se o valor raiz é um objeto.
///
/// Configurações precisam possuir uma raiz nomeada:
///
/// ```json
/// {
///     "server": {}
/// }
/// ```
pub(crate) fn ensure_object(
    value: Value,
) -> ConfigResult<Value> {

    match value {

        Value::Object(_) => {
            Ok(value)
        }


        _ => {

            Err(
                ConfigError::InvalidConfiguration(
                    "configuration root must be an object"
                        .into(),
                )
            )

        }
    }
}