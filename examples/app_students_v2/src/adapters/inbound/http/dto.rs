use crate::domain::{Aluno, Matricula, Professor, Turma};

#[derive(Debug, serde::Serialize)]
pub struct AlunoResponse {
    pub id: String,
    pub nome: String,
    pub email: String,
}

impl From<Aluno> for AlunoResponse {
    fn from(value: Aluno) -> Self {
        Self {
            id: value.id.unwrap_or_default(),
            nome: value.nome,
            email: value.email,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ProfessorResponse {
    pub id: String,
    pub nome: String,
    pub email: String,
}

impl From<Professor> for ProfessorResponse {
    fn from(value: Professor) -> Self {
        Self {
            id: value.id.unwrap_or_default(),
            nome: value.nome,
            email: value.email,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct TurmaResponse {
    pub id: String,
    pub nome: String,
    pub professor_id: String,
}

impl From<Turma> for TurmaResponse {
    fn from(value: Turma) -> Self {
        Self {
            id: value.id.unwrap_or_default(),
            nome: value.nome,
            professor_id: value.professor_id,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct MatriculaResponse {
    pub id: String,
    pub aluno_id: String,
    pub turma_id: String,
}

impl From<Matricula> for MatriculaResponse {
    fn from(value: Matricula) -> Self {
        Self {
            id: value.id.unwrap_or_default(),
            aluno_id: value.aluno_id,
            turma_id: value.turma_id,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct CriarAlunoRequest {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AtualizarAlunoRequest {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CriarProfessorRequest {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AtualizarProfessorRequest {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CriarTurmaRequest {
    pub nome: String,
    pub professor_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct AtualizarTurmaRequest {
    pub nome: String,
    pub professor_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CriarMatriculaRequest {
    pub aluno_id: String,
    pub turma_id: String,
}
