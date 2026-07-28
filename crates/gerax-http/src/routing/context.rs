
use std::sync::Arc;
use urlencoding;

use std::collections::HashMap;

use super::{Request, ExtractError};
use serde::de::DeserializeOwned;
use serde_urlencoded;
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

#[derive(Clone)]
pub struct Extensions;
impl Extensions {
    pub fn new() -> Self {
        Self
    }
}

pub struct Context<State> {
    pub state: Arc<State>,
    pub request: Request,
    pub params: PathParams,
    pub extensions: Extensions,
}

impl<State> Clone for Context<State> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            request: self.request.clone(),
            params: self.params.clone(),
            extensions: self.extensions.clone(),
        }
    }
}

impl<State> Context<State> {

    pub fn new(state: Arc<State>, request: Request) -> Self {
        Self {
            state,
            params: PathParams::new(HashMap::new()),
            extensions: Extensions::new(),
            request,
        }
    }

    pub fn state(&self) -> Arc<State> {
        Arc::clone(&self.state)
    }

    pub fn params(&self) -> &PathParams {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut PathParams {
        &mut self.params
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn request_mut(&mut self) -> &mut Request {
        &mut self.request
    }
}

