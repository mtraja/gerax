use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

    pub fn insert<T>(&self, val: T)
    where
        T: Send + Sync + 'static,
    {
        self.map
            .lock()
            .unwrap()
            .insert(TypeId::of::<T>(), Arc::new(val));
    }

    /// Obtém uma cópia da referência compartilhada armazenada para `T`.
    ///
    /// Para valores mutáveis, armazene um `Mutex<T>` ou `RwLock<T>` e adquira
    /// o lock explicitamente no chamador.
    pub fn get<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.map
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())?
            .clone()
            .downcast::<T>()
            .ok()
    }

    /// Remove e devolve a referência compartilhada armazenada para `T`.
    pub fn remove<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
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
