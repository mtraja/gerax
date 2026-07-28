
use std::sync::Arc;
use urlencoding;

use std::collections::HashMap;

use super::{Request, ExtractError};
pub struct PathParams {
    params: HashMap<String, String>,
}

impl PathParams {
    pub fn new(params: HashMap<String, String>) -> Self {
        Self { params }
    }

    pub fn deserialize<T>(&self) -> Result<T, ExtractError>
    where
        T: serde::de::DeserializeOwned,
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

