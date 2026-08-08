//! Extensões opcionais do RPC.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Extensões opcionais para RPC.
///
/// Permite anexar dados tipados ao contexto ou resposta
/// sem acoplamento ao protocolo.
#[derive(Debug, Default)]
pub struct RpcExtensions {
    inner: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl RpcExtensions {
    /// Cria um novo `RpcExtensions` vazio.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Insere uma extensão tipada.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.inner.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Obtém uma extensão tipada.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.inner
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
    }

    /// Remove uma extensão tipada.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.inner
            .remove(&TypeId::of::<T>())
            .and_then(|v| v.downcast::<T>().ok())
            .map(|v| *v)
    }
}
