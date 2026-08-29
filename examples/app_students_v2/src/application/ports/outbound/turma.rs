use async_trait::async_trait;

use crate::application::errors::ApplicationError;
use crate::domain::Turma;

#[async_trait]
pub trait TurmaRepository: Send + Sync {
    async fn save(&self, turma: &Turma) -> Result<Turma, ApplicationError>;
    async fn find(&self, id: &str) -> Result<Option<Turma>, ApplicationError>;
    async fn list(&self) -> Result<Vec<Turma>, ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
}
