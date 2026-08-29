use async_trait::async_trait;

use crate::application::errors::ApplicationError;
use crate::domain::Matricula;

#[async_trait]
pub trait MatriculaRepository: Send + Sync {
    async fn save(&self, matricula: &Matricula) -> Result<Matricula, ApplicationError>;
    async fn find(&self, id: &str) -> Result<Option<Matricula>, ApplicationError>;
    async fn list(&self) -> Result<Vec<Matricula>, ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
}
