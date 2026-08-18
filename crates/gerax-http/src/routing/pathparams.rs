use std::collections::HashMap;
use serde::de::DeserializeOwned;
use serde_urlencoded;
use urlencoding;

use super::ExtractError;

#[derive(Clone)]
pub struct PathParams {
    params: HashMap<String, String>,
}

impl PathParams {
    pub fn new(params: HashMap<String, String>) -> Self {
        Self { params }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.params.insert(key, value);
    }

    pub fn deserialize<T>(&self) -> Result<T, ExtractError>
    where
        T: DeserializeOwned,
    {
        serde_urlencoded::from_str(&self.to_query_string())
            .map_err(|err| ExtractError::Deserialize(err.to_string()))
    }

    fn to_query_string(&self) -> String {
        self.params
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}={}",
                    urlencoding::encode(key),
                    urlencoding::encode(value)
                )
            })
            .collect::<Vec<_>>()
            .join("&")
    }
}
