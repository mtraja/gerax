use std::sync::Arc;

use gerax_db::Repository;
use gerax_core::Entity;

use crate::websocket::handler::WsContext;

#[derive(Clone)]
pub struct WsRepository<T>(pub Arc<dyn Repository<T> + Send + Sync>);

impl<T> WsRepository<T> {
    pub fn new(repo: Arc<dyn Repository<T> + Send + Sync>) -> Self {
        Self(repo)
    }

    pub fn inner(&self) -> Arc<dyn Repository<T> + Send + Sync> {
        Arc::clone(&self.0)
    }
}

pub trait RepositoryResolver<State> {
    fn register_repository<T: Entity + Send + Sync + 'static>(&self, repository: WsRepository<T>);
    fn repository<T: Entity + Send + Sync + 'static>(&self) -> Result<WsRepository<T>, WsError>;
}

#[derive(Debug)]
pub struct WsError(String);

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for WsError {}

impl<State> RepositoryResolver<State> for WsContext<State> {
    fn register_repository<T: Entity + Send + Sync + 'static>(&self, repository: WsRepository<T>) {
        self.extensions.insert(repository);
    }

    fn repository<T: Entity + Send + Sync + 'static>(&self) -> Result<WsRepository<T>, WsError> {
        self.extensions
            .get::<WsRepository<T>>()
            .map(|arc| arc.as_ref().clone())
            .ok_or_else(|| WsError(format!("Repository for {} not found", T::collection_name())))
    }
}
