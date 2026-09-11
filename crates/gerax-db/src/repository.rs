use async_trait::async_trait;
use gerax_core::Entity;

use crate::DbError;

#[async_trait]
pub trait Repository<T>: Send + Sync + 'static 
where T: Entity + Send + Sync + 'static
{
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, DbError>;
    async fn find_all(&self) -> Result<Vec<T>, DbError>;
    async fn insert(&self, entity: T) -> Result<T, DbError>;
    async fn update(&self, entity: T) -> Result<(), DbError>;
    async fn delete(&self, id: &str) -> Result<(), DbError>;
}


#[async_trait]
pub trait EntityRepository: Send + Sync + 'static {
    type Entity: Send + Sync + 'static;

    async fn find_by_id(&self, id: &str) -> Result<Option<Self::Entity>, DbError>;
    async fn find_all(&self) -> Result<Vec<Self::Entity>, DbError>;
    async fn insert(&self, entity: Self::Entity) -> Result<Self::Entity, DbError>;
    async fn update(&self, entity: Self::Entity) -> Result<(), DbError>;
    async fn delete(&self, id: &str) -> Result<(), DbError>;
}
