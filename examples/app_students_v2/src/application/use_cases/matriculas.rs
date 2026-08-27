use std::sync::Arc;

use crate::application::errors::ApplicationError;
use crate::application::ports::aluno::AlunoRepository;
use crate::application::ports::aluno_por_turma::AlunoPorTurmaQuery;
use crate::application::ports::matricula::MatriculaRepository;
use crate::application::ports::turma::TurmaRepository;
use crate::domain::{Aluno, Matricula};

#[derive(Debug, Clone)]
pub struct CriarMatricula {
    pub aluno_id: String,
    pub turma_id: String,
}

#[derive(Debug, Clone)]
pub struct DeletarMatricula(pub String);

#[derive(Debug, Clone)]
pub struct ObterMatricula(pub String);

pub struct MatriculaUseCases<R1, R2, R3, Q> {
    repo: Arc<R1>,
    aluno_repo: Arc<R2>,
    turma_repo: Arc<R3>,
    query: Arc<Q>,
}

impl<R1, R2, R3, Q> MatriculaUseCases<R1, R2, R3, Q>
where
    R1: MatriculaRepository,
    R2: AlunoRepository,
    R3: TurmaRepository,
    Q: AlunoPorTurmaQuery,
{
    pub fn new(repo: Arc<R1>, aluno_repo: Arc<R2>, turma_repo: Arc<R3>, query: Arc<Q>) -> Self {
        Self {
            repo,
            aluno_repo,
            turma_repo,
            query,
        }
    }

    pub async fn criar(&self, input: CriarMatricula) -> Result<Matricula, ApplicationError> {
        let aluno = self
            .aluno_repo
            .find(&input.aluno_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found(format!("aluno {}", input.aluno_id)))?;

        let _turma = self
            .turma_repo
            .find(&input.turma_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found(format!("turma {}", input.turma_id)))?;

        let alunos_na_turma = self.query.listar_alunos_por_turma(&input.turma_id).await?;
        if alunos_na_turma
            .iter()
            .any(|a| a.id.as_deref() == Some(&input.aluno_id))
        {
            return Err(ApplicationError::business_rule(format!(
                "matrícula duplicada: aluno {} já está na turma {}",
                input.aluno_id, input.turma_id
            )));
        }

        let matricula = Matricula::nova(aluno.id.unwrap_or(input.aluno_id), input.turma_id)?;
        self.repo.save(&matricula).await
    }

    pub async fn obter(
        &self,
        input: ObterMatricula,
    ) -> Result<Option<Matricula>, ApplicationError> {
        self.repo.find(&input.0).await
    }

    pub async fn listar(&self) -> Result<Vec<Matricula>, ApplicationError> {
        self.repo.list().await
    }

    pub async fn deletar(&self, input: DeletarMatricula) -> Result<(), ApplicationError> {
        self.repo.delete(&input.0).await
    }

    pub async fn listar_alunos_por_turma(
        &self,
        turma_id: &str,
    ) -> Result<Vec<Aluno>, ApplicationError> {
        self.query.listar_alunos_por_turma(turma_id).await
    }
}
