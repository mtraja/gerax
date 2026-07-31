use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use urlencoding;

use super::{ExtractError, Request};
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
pub struct Extensions {
    map: Arc<Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl Extensions {
    pub fn new() -> Self {
        Self {
            map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert<T: Send + Sync + 'static>(&self, val: T) {
        self.map
            .lock()
            .unwrap()
            .insert(TypeId::of::<T>(), Arc::new(val));
    }

    /// Obtém uma cópia da referência compartilhada armazenada para `T`.
    ///
    /// Para valores mutáveis, armazene um `Mutex<T>` ou `RwLock<T>` e adquira
    /// o lock explicitamente no chamador.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())?
            .clone()
            .downcast::<T>()
            .ok()
    }

    /// Remove e devolve a referência compartilhada armazenada para `T`.
    pub fn remove<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .lock()
            .unwrap()
            .remove(&TypeId::of::<T>())?
            .downcast::<T>()
            .ok()
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Self::new()
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
