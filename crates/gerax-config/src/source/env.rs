use std::{collections::HashMap, path::PathBuf};

use dotenvy::from_path_iter;
use serde_json::{Map, Value};

use crate::{
    document::ConfigDocument,
    error::{ConfigError, ConfigResult},
    source::ConfigSource,
};

/// Representa uma fonte em .Env
#[derive(Debug, Clone)]
pub struct EnvSource {
    path: Option<PathBuf>,
}

impl EnvSource {
    /// Lê somente variáveis do processo.
    pub fn system() -> Self {
        Self { path: None }
    }

    /// Lê de um arquivo .env.
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
        }
    }

    /// Atalho para ".env".
    pub fn dotenv() -> Self {
        Self {
            path: Some(PathBuf::from(".env")),
        }
    }

    fn variables(&self) -> ConfigResult<HashMap<String, String>> {
        match &self.path {
            Some(path) => {
                let iter = from_path_iter(path).map_err(|e| ConfigError::Env {
                    path: path.display().to_string(),

                    source: e,
                })?;

                let mut vars = HashMap::new();

                for item in iter {
                    let (key, value) = item.map_err(|e| ConfigError::Env {
                        path: path.display().to_string(),
                        source: e,
                    })?;

                    vars.insert(key, value);
                }

                Ok(vars)
            }

            None => Ok(std::env::vars().collect()),
        }
    }
}

impl Default for EnvSource {
    fn default() -> Self {
        Self::dotenv()
    }
}

impl ConfigSource for EnvSource {
    fn name(&self) -> &'static str {
        "env"
    }

    fn load(&self) -> ConfigResult<ConfigDocument> {
        let vars = self.variables()?;

        let mut root = Map::new();

        for (key, value) in vars {
            let path = normalize_key(&key);

            insert_value(&mut root, &path, parse_value(&value));
        }

        ConfigDocument::from_value(Value::Object(root))
    }
}

/// Converte:
///
/// SERVER__DATABASE__URL
///
/// em:
///
/// ["server","database","url"]
fn normalize_key(key: &str) -> Vec<String> {
    key.split("__").map(|x| x.to_lowercase()).collect()
}

fn insert_value(map: &mut Map<String, Value>, path: &[String], value: Value) {
    if path.len() == 1 {
        map.insert(path[0].clone(), value);

        return;
    }

    let entry = map
        .entry(path[0].clone())
        .or_insert_with(|| Value::Object(Map::new()));

    let child = entry
        .as_object_mut()
        .expect("configuration node must be object");

    insert_value(child, &path[1..], value);
}

/// Conversão automática.
///
/// "true" -> bool
/// "8080" -> number
/// "1.5" -> float
/// resto -> string
fn parse_value(value: &str) -> Value {
    if let Ok(v) = value.parse::<bool>() {
        return Value::Bool(v);
    }

    if let Ok(v) = value.parse::<i64>() {
        return Value::Number(v.into());
    }

    if let Ok(v) = value.parse::<f64>() {
        return serde_json::Number::from_f64(v)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(value.into()));
    }

    Value::String(value.into())
}
