//! Carregamento de dados em lote para resolvers GraphQL.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::try_join_all;
use gerax_http::routing::context::Context;
use tokio::sync::{Mutex, RwLock, oneshot};

use crate::GraphqlError;

/// Fonte assíncrona de valores que podem ser buscados em lote.
///
/// A implementação deve devolver um mapa para preservar a associação entre
/// cada chave solicitada e seu valor. Chaves ausentes são convertidas em
/// [`GraphqlError::Execution`] pelo [`DataLoader`].
#[async_trait]
pub trait BatchLoader<State, Key, Value>: Send + Sync + 'static {
    /// Busca valores para as chaves distintas solicitadas em um lote.
    async fn load(
        &self,
        state: Arc<State>,
        keys: Vec<Key>,
    ) -> Result<HashMap<Key, Value>, GraphqlError>;
}

type Waiter<Value> = oneshot::Sender<Result<Value, GraphqlError>>;

struct DataLoaderInner<Key, Value, State, Loader> {
    loader: Loader,
    cache: RwLock<HashMap<Key, Value>>,
    pending: Mutex<HashMap<Key, Vec<Waiter<Value>>>>,
    dispatch_scheduled: Mutex<bool>,
    _state: std::marker::PhantomData<fn(State)>,
}

/// Carregador assíncrono que agrupa requisições simultâneas por chave.
///
/// O carregador deduplica chaves pendentes, executa uma única consulta para o
/// lote e mantém um cache em memória. Ele deve ser compartilhado por `Arc`
/// durante o ciclo de vida desejado (normalmente, uma requisição GraphQL).
///
/// # Exemplo
///
/// ```ignore
/// let loader = DataLoader::new(UserBatchLoader);
/// loader.insert_into_context(&context);
/// let user = loader.load(&context, user_id).await?;
/// ```
pub struct DataLoader<Key, Value, State, Loader> {
    inner: Arc<DataLoaderInner<Key, Value, State, Loader>>,
}

impl<Key, Value, State, Loader> Clone for DataLoader<Key, Value, State, Loader> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<Key, Value, State, Loader> DataLoader<Key, Value, State, Loader>
where
    Key: Clone + Eq + Hash + Send + Sync + 'static,
    Value: Clone + Send + Sync + 'static,
    State: Send + Sync + 'static,
    Loader: BatchLoader<State, Key, Value>,
{
    /// Cria um carregador a partir de uma fonte de lote.
    pub fn new(loader: Loader) -> Self {
        Self {
            inner: Arc::new(DataLoaderInner {
                loader,
                cache: RwLock::new(HashMap::new()),
                pending: Mutex::new(HashMap::new()),
                dispatch_scheduled: Mutex::new(false),
                _state: std::marker::PhantomData,
            }),
        }
    }

    /// Armazena uma cópia compartilhada deste loader nas extensões do contexto.
    pub fn insert_into_context(&self, context: &Context<State>) {
        context.extensions().insert(self.clone());
    }

    /// Recupera um loader previamente inserido nas extensões do contexto.
    pub fn from_context(context: &Context<State>) -> Option<Arc<Self>> {
        context.extensions().get::<Self>()
    }

    /// Carrega uma chave, usando o cache ou agrupando-a com cargas simultâneas.
    pub async fn load(&self, context: &Context<State>, key: Key) -> Result<Value, GraphqlError> {
        if let Some(value) = self.inner.cache.read().await.get(&key).cloned() {
            return Ok(value);
        }

        let handle = tokio::runtime::Handle::try_current().map_err(|error| {
            GraphqlError::Internal(format!("DataLoader requires a Tokio runtime: {error}"))
        })?;

        let (sender, receiver) = oneshot::channel();
        let should_schedule = {
            let mut pending = self.inner.pending.lock().await;
            pending.entry(key).or_default().push(sender);

            let mut scheduled = self.inner.dispatch_scheduled.lock().await;
            if *scheduled {
                false
            } else {
                *scheduled = true;
                true
            }
        };

        if should_schedule {
            let loader = self.clone();
            let state = context.state();
            handle.spawn(async move {
                tokio::task::yield_now().await;
                loader.dispatch(state).await;
            });
        }

        receiver.await.map_err(|_| {
            GraphqlError::Internal("DataLoader dispatcher stopped before completing a load".into())
        })?
    }

    /// Carrega várias chaves preservando a ordem de entrada.
    ///
    /// Todas as chaves não presentes no cache são incluídas no mesmo lote.
    pub async fn load_many(
        &self,
        context: &Context<State>,
        keys: Vec<Key>,
    ) -> Result<Vec<Value>, GraphqlError> {
        try_join_all(keys.into_iter().map(|key| self.load(context, key))).await
    }

    /// Limpa todos os valores do cache.
    pub async fn clear_cache(&self) {
        self.inner.cache.write().await.clear();
    }

    /// Remove uma chave do cache.
    pub async fn clear(&self, key: &Key) {
        self.inner.cache.write().await.remove(key);
    }

    /// Retorna a quantidade de entradas armazenadas no cache.
    pub async fn cache_size(&self) -> usize {
        self.inner.cache.read().await.len()
    }

    async fn dispatch(&self, state: Arc<State>) {
        let pending = {
            let mut pending = self.inner.pending.lock().await;
            let pending = std::mem::take(&mut *pending);
            *self.inner.dispatch_scheduled.lock().await = false;
            pending
        };

        if pending.is_empty() {
            return;
        }

        let keys = pending.keys().cloned().collect();
        match self.inner.loader.load(state, keys).await {
            Ok(values) => {
                self.inner.cache.write().await.extend(values.clone());
                for (key, waiters) in pending {
                    let result = values.get(&key).cloned().ok_or_else(|| {
                        GraphqlError::Execution(
                            "batch loader did not return a value for a requested key".into(),
                        )
                    });
                    for waiter in waiters {
                        let _ = waiter.send(result.clone());
                    }
                }
            }
            Err(error) => {
                for waiters in pending.into_values() {
                    for waiter in waiters {
                        let _ = waiter.send(Err(error.clone()));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gerax_http::routing::{Context, HttpMethod, Request};

    use super::*;

    #[derive(Default)]
    struct TestBatchLoader {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl BatchLoader<(), u32, String> for Arc<TestBatchLoader> {
        async fn load(
            &self,
            _state: Arc<()>,
            keys: Vec<u32>,
        ) -> Result<HashMap<u32, String>, GraphqlError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(keys
                .into_iter()
                .map(|key| (key, format!("value-{key}")))
                .collect())
        }
    }

    fn context() -> Context<()> {
        Context::new(
            Arc::new(()),
            Request::new(HttpMethod::Get, "/graphql".into(), Vec::new()),
        )
    }

    #[tokio::test]
    async fn batches_distinct_and_duplicate_keys() {
        let source = Arc::new(TestBatchLoader::default());
        let loader = DataLoader::new(Arc::clone(&source));
        let context = context();

        let values = loader.load_many(&context, vec![1, 2, 1]).await;

        assert_eq!(
            values,
            Ok(vec!["value-1".into(), "value-2".into(), "value-1".into()])
        );
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn uses_cache_and_context_extensions() {
        let source = Arc::new(TestBatchLoader::default());
        let loader = DataLoader::new(Arc::clone(&source));
        let context = context();
        loader.insert_into_context(&context);

        let stored = DataLoader::<u32, String, (), Arc<TestBatchLoader>>::from_context(&context);
        assert!(stored.is_some());
        assert_eq!(loader.load(&context, 4).await, Ok("value-4".into()));
        assert_eq!(loader.load(&context, 4).await, Ok("value-4".into()));
        assert_eq!(loader.cache_size().await, 1);
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn reports_keys_missing_from_a_batch_result() {
        struct EmptyLoader;

        #[async_trait]
        impl BatchLoader<(), u32, String> for EmptyLoader {
            async fn load(
                &self,
                _state: Arc<()>,
                _keys: Vec<u32>,
            ) -> Result<HashMap<u32, String>, GraphqlError> {
                Ok(HashMap::new())
            }
        }

        let loader = DataLoader::new(EmptyLoader);
        let error = loader.load(&context(), 1).await;
        assert!(matches!(error, Err(GraphqlError::Execution(_))));
    }
}
