use async_trait::async_trait;
use crate::core::entity::Entity;
use crate::core::error::AppError;

#[async_trait]
pub trait Repository<T: Entity>: Send + Sync + 'static {
    async fn create(&self, item: T) -> Result<T, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, AppError>;
    async fn find_all(&self) -> Result<Vec<T>, AppError>;
    async fn update(&self, id: &str, item: T) -> Result<Option<T>, AppError>;
    async fn delete(&self, id: &str) -> Result<bool, AppError>;
}