use crate::GraphqlError;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache interno para queries persistidas.
pub struct PersistedQueryCache {
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl PersistedQueryCache {
    /// Cria um novo cache vazio.
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Calcula o hash SHA256 de uma query.
    pub fn hash_query(query: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Registra uma query no cache.
    pub async fn register(&self, hash: String, query: String) {
        let mut cache = self.cache.write().await;
        cache.insert(hash, query);
    }

    /// Busca uma query no cache pelo hash.
    pub async fn get(&self, hash: &str) -> Option<String> {
        let cache = self.cache.read().await;
        cache.get(hash).cloned()
    }

    /// Remove uma query do cache.
    pub async fn remove(&self, hash: &str) -> Option<String> {
        let mut cache = self.cache.write().await;
        cache.remove(hash)
    }

    /// Retorna o número de queries no cache.
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Verifica se o cache está vazio.
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
    }

    /// Limpa todo o cache.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Gerenciador de queries persistidas.
///
/// Utiliza hash SHA256 para identificar queries
/// e cache interno para armazenar as queries originais.
pub struct PersistedQueryManager {
    cache: PersistedQueryCache,
}

impl PersistedQueryManager {
    /// Cria um novo gerenciador de queries persistidas.
    pub fn new() -> Self {
        Self {
            cache: PersistedQueryCache::new(),
        }
    }

    /// Registra uma query no cache e retorna seu hash SHA256.
    pub async fn register(&self, query: &str) -> String {
        let hash = PersistedQueryCache::hash_query(query);
        self.cache.register(hash.clone(), query.to_string()).await;
        hash
    }

    /// Valida uma query persistida pelo hash.
    ///
    /// Retorna a query original se encontrada no cache,
    /// ou um erro se a query não existir.
    pub async fn validate(&self, hash: &str, query: Option<&str>) -> Result<String, GraphqlError> {
        let cached_query = self.cache.get(hash).await;

        match (cached_query, query) {
            (Some(cached), Some(provided)) => {
                if cached == provided {
                    Ok(cached)
                } else {
                    Err(GraphqlError::PersistedQuery(
                        "query mismatch for persisted hash".to_string(),
                    ))
                }
            }
            (Some(cached), None) => Ok(cached),
            (None, Some(provided)) => {
                self.register(provided).await;
                Ok(provided.to_string())
            }
            (None, None) => Err(GraphqlError::PersistedQuery(
                "no query provided and hash not found in cache".to_string(),
            )),
        }
    }

    /// Remove uma query do cache.
    pub async fn remove(&self, hash: &str) -> Option<String> {
        self.cache.remove(hash).await
    }

    /// Retorna o número de queries persistidas no cache.
    pub async fn len(&self) -> usize {
        self.cache.len().await
    }

    /// Verifica se não há queries persistidas no cache.
    pub async fn is_empty(&self) -> bool {
        self.cache.is_empty().await
    }
}

impl Default for PersistedQueryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PersistedQueryCache {
    fn default() -> Self {
        Self::new()
    }
}
