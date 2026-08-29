use async_trait::async_trait;

use crate::application::errors::ApplicationError;
use crate::domain::Aluno;

#[async_trait]
pub trait AlunoRepository: Send + Sync {
    async fn save(&self, aluno: &Aluno) -> Result<Aluno, ApplicationError>;
    async fn find(&self, id: &str) -> Result<Option<Aluno>, ApplicationError>;
    async fn list(&self) -> Result<Vec<Aluno>, ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
}
