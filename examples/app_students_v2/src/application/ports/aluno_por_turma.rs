use async_trait::async_trait;

use crate::application::errors::ApplicationError;
use crate::domain::Aluno;

#[async_trait]
pub trait AlunoPorTurmaQuery: Send + Sync {
    async fn listar_alunos_por_turma(&self, turma_id: &str)
    -> Result<Vec<Aluno>, ApplicationError>;
}
