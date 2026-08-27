use std::collections::HashMap;

use super::HttpMethod;

#[derive(Clone)]
pub struct HeaderMap {
    values: HashMap<String, String>,
}

impl HeaderMap {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .get(&name.to_ascii_lowercase())
            .map(|s| s.as_str())
    }

    pub fn insert(&mut self, name: impl AsRef<str>, value: impl Into<String>) -> Option<String> {
        self.values
            .insert(name.as_ref().to_ascii_lowercase(), value.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.values.iter()
    }
}

impl Default for HeaderMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Request {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HeaderMap,
    pub query: String,
    pub body: Vec<u8>,
}

impl Request {
    pub fn new(method: HttpMethod, path: String, body: Vec<u8>) -> Self {
        let (path, query) = match path.find('?') {
            Some(index) => (path[..index].to_string(), path[index + 1..].to_string()),
            None => (path, String::new()),
        };
        Self {
            method,
            path,
            body,
            query,
            headers: HeaderMap::new(),
        }
    }

    pub fn method(&self) -> &HttpMethod {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
}

/*
pub struct Request<State> {
    state: Arc<State>,
    method: Method,
    uri: Uri,
    headers: Headers,
    body: Body,
}
---
pub struct Request {
    method: Method,
    path: String,
    headers: HeaderMap,
    query: String,
    body: Vec<u8>,
}

pub struct Context<State> {
    state: Arc<State>,
    request: Request,
    params: PathParams,
    extensions: Extensions,
}

impl<State> Context<State> {

    pub fn state(&self) -> &State {
        self.state.as_ref()
    }

    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn params(&self) -> &PathParams {
        &self.params
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

}

*/
