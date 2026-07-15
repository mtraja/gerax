use async_trait::async_trait;

use crate::DbError;

#[async_trait]
pub trait Connection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized;
    async fn ping(&self) -> Result<(), DbError>;
}
