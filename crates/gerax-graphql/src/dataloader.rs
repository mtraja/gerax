use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::GraphqlError;

/// Carregador de dados em lote para resolver o problema N+1.
///
/// O DataLoader batching múltiplas requisições em uma única
/// chamada de banco de dados e cacheia os resultados.
///
/// Compatível com `Context<State>`.
pub struct DataLoader<Key, Value, State> {
    batch_fn: Arc<dyn Fn(Vec<Key>) -> Vec<Value> + Send + Sync>,
    cache: Arc<RwLock<HashMap<Key, Value>>>,
    _marker: std::marker::PhantomData<(Value, State)>,
}

impl<Key, Value, State> DataLoader<Key, Value, State>
where
    Key: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    Value: Clone + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    /// Cria um novo DataLoader com a função de batch especificada.
    pub fn new<F>(batch_fn: F) -> Self
    where
        F: Fn(Vec<Key>) -> Vec<Value> + Send + Sync + 'static,
    {
        Self {
            batch_fn: Arc::new(batch_fn),
            cache: Arc::new(RwLock::new(HashMap::new())),
            _marker: std::marker::PhantomData,
        }
    }

    /// Carrega um valor pelo key, usando cache quando disponível.
    pub async fn load(&self, key: Key) -> Result<Value, GraphqlError> {
        {
            let cache = self.cache.read().await;
            if let Some(value) = cache.get(&key) {
                return Ok(value.clone());
            }
        }

        let values = (self.batch_fn)(vec![key.clone()]);
        if let Some(value) = values.into_iter().next() {
            let mut cache = self.cache.write().await;
            cache.insert(key, value.clone());
            Ok(value)
        } else {
            Err(GraphqlError::Execution(
                "value not found for key".to_string(),
            ))
        }
    }

    /// Carrega múltiplos valores de uma vez em batch.
    pub async fn load_many(&self, keys: Vec<Key>) -> Result<Vec<Value>, GraphqlError> {
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(self.load(key).await?);
        }
        Ok(results)
    }

    /// Limpa o cache do DataLoader.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Remove uma entrada específica do cache.
    pub async fn clear(&self, key: &Key) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    /// Retorna o número de entradas no cache.
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

/// DataLoader integrado com o contexto GraphQL.
///
/// Permite acessar o DataLoader através do `Context<State>`.
pub struct ContextDataLoader<State> {
    loader: Arc<DataLoader<String, serde_json::Value, State>>,
}

impl<State: Send + Sync + 'static> ContextDataLoader<State> {
    /// Cria um novo ContextDataLoader.
    pub fn new(
        loader: DataLoader<String, serde_json::Value, State>,
    ) -> Self {
        Self {
            loader: Arc::new(loader),
        }
    }

    /// Carrega um valor pelo key usando o DataLoader.
    pub async fn load(&self, key: &str) -> Result<serde_json::Value, GraphqlError> {
        self.loader.load(key.to_string()).await
    }

    /// Limpa o cache do DataLoader.
    pub async fn clear_cache(&self) {
        self.loader.clear_cache().await;
    }
}