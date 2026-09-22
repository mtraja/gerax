use std::sync::Arc;

use crate::application::errors::ApplicationError;
use crate::application::ports::outbound::turma::TurmaRepository;
use crate::domain::Turma;

#[derive(Debug, Clone)]
pub struct CriarTurma {
    pub nome: String,
    pub professor_id: String,
}

#[derive(Debug, Clone)]
pub struct AtualizarTurma {
    pub id: String,
    pub nome: String,
    pub professor_id: String,
}

#[derive(Debug, Clone)]
pub struct DeletarTurma(pub String);

#[derive(Debug, Clone)]
pub struct ObterTurma(pub String);

pub struct TurmaUseCases<R> {
    repo: Arc<R>,
}

impl<R: TurmaRepository> TurmaUseCases<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn criar(&self, input: CriarTurma) -> Result<Turma, ApplicationError> {
        let turma = Turma::nova(input.nome, input.professor_id)?;
        self.repo.save(&turma).await
    }

    pub async fn atualizar(&self, input: AtualizarTurma) -> Result<Turma, ApplicationError> {
        let mut turma = self
            .repo
            .find(&input.id)
            .await?
            .ok_or_else(|| ApplicationError::not_found(format!("turma {}", input.id)))?;
        turma.atualizar(input.nome, input.professor_id)?;
        self.repo.save(&turma).await
    }

    pub async fn obter(&self, input: ObterTurma) -> Result<Option<Turma>, ApplicationError> {
        self.repo.find(&input.0).await
    }

    pub async fn listar(&self) -> Result<Vec<Turma>, ApplicationError> {
        self.repo.list().await
    }

    pub async fn deletar(&self, input: DeletarTurma) -> Result<(), ApplicationError> {
        self.repo.delete(&input.0).await
    }
}
