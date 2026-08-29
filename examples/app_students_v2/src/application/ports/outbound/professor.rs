use async_trait::async_trait;

use crate::application::errors::ApplicationError;
use crate::domain::Professor;

#[async_trait]
pub trait ProfessorRepository: Send + Sync {
    async fn save(&self, professor: &Professor) -> Result<Professor, ApplicationError>;
    async fn find(&self, id: &str) -> Result<Option<Professor>, ApplicationError>;
    async fn list(&self) -> Result<Vec<Professor>, ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
}
