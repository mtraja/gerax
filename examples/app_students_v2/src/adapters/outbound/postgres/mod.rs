use std::sync::Arc;

use async_trait::async_trait;
use gerax_app::{PostgresConnection, Repository};
use gerax_core::Entity;
use gerax_postgres::PostgresRepository;
use sqlx::Row;

use crate::application::errors::ApplicationError;
use crate::application::ports::{
    AlunoPorTurmaQuery, AlunoRepository, MatriculaRepository, ProfessorRepository, TurmaRepository,
};
use crate::domain::{Aluno, Matricula, Professor, Turma};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlunoEntity {
    pub id: Option<String>,
    pub nome: String,
    pub email: String,
}

impl From<AlunoEntity> for Aluno {
    fn from(value: AlunoEntity) -> Self {
        Self {
            id: value.id,
            nome: value.nome,
            email: value.email,
        }
    }
}

impl From<Aluno> for AlunoEntity {
    fn from(value: Aluno) -> Self {
        Self {
            id: value.id,
            nome: value.nome,
            email: value.email,
        }
    }
}

impl Entity for AlunoEntity {
    fn collection_name() -> &'static str {
        "alunos"
    }
    fn id(&self) -> Option<String> {
        self.id.clone()
    }
    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfessorEntity {
    pub id: Option<String>,
    pub nome: String,
    pub email: String,
}

impl From<ProfessorEntity> for Professor {
    fn from(value: ProfessorEntity) -> Self {
        Self {
            id: value.id,
            nome: value.nome,
            email: value.email,
        }
    }
}

impl From<Professor> for ProfessorEntity {
    fn from(value: Professor) -> Self {
        Self {
            id: value.id,
            nome: value.nome,
            email: value.email,
        }
    }
}

impl Entity for ProfessorEntity {
    fn collection_name() -> &'static str {
        "professores"
    }
    fn id(&self) -> Option<String> {
        self.id.clone()
    }
    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TurmaEntity {
    pub id: Option<String>,
    pub nome: String,
    pub professor_id: String,
}

impl From<TurmaEntity> for Turma {
    fn from(value: TurmaEntity) -> Self {
        Self {
            id: value.id,
            nome: value.nome,
            professor_id: value.professor_id,
        }
    }
}

impl From<Turma> for TurmaEntity {
    fn from(value: Turma) -> Self {
        Self {
            id: value.id,
            nome: value.nome,
            professor_id: value.professor_id,
        }
    }
}

impl Entity for TurmaEntity {
    fn collection_name() -> &'static str {
        "turmas"
    }
    fn id(&self) -> Option<String> {
        self.id.clone()
    }
    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MatriculaEntity {
    pub id: Option<String>,
    pub aluno_id: String,
    pub turma_id: String,
}

impl From<MatriculaEntity> for Matricula {
    fn from(value: MatriculaEntity) -> Self {
        Self {
            id: value.id,
            aluno_id: value.aluno_id,
            turma_id: value.turma_id,
        }
    }
}

impl From<Matricula> for MatriculaEntity {
    fn from(value: Matricula) -> Self {
        Self {
            id: value.id,
            aluno_id: value.aluno_id,
            turma_id: value.turma_id,
        }
    }
}

impl Entity for MatriculaEntity {
    fn collection_name() -> &'static str {
        "matriculas"
    }
    fn id(&self) -> Option<String> {
        self.id.clone()
    }
    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

pub struct PostgresAlunoRepository {
    repo: PostgresRepository<AlunoEntity>,
}

impl PostgresAlunoRepository {
    pub fn new(connection: Arc<PostgresConnection>) -> Self {
        Self {
            repo: PostgresRepository::new(connection),
        }
    }

    pub async fn create_table(&self) -> Result<(), ApplicationError> {
        self.repo
            .create_table()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("create_table alunos", e))
    }
}

#[async_trait]
impl AlunoRepository for PostgresAlunoRepository {
    async fn save(&self, aluno: &Aluno) -> Result<Aluno, ApplicationError> {
        let entity: AlunoEntity = aluno.clone().into();
        let saved = self
            .repo
            .insert(entity)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("insert aluno", e))?;
        Ok(saved.into())
    }

    async fn find(&self, id: &str) -> Result<Option<Aluno>, ApplicationError> {
        let found = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("find aluno", e))?;
        Ok(found.map(|e| e.into()))
    }

    async fn list(&self) -> Result<Vec<Aluno>, ApplicationError> {
        let items = self
            .repo
            .find_all()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("list alunos", e))?;
        Ok(items.into_iter().map(|e| e.into()).collect())
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.repo
            .delete(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("delete aluno", e))
    }
}

pub struct PostgresProfessorRepository {
    repo: PostgresRepository<ProfessorEntity>,
}

impl PostgresProfessorRepository {
    pub fn new(connection: Arc<PostgresConnection>) -> Self {
        Self {
            repo: PostgresRepository::new(connection),
        }
    }

    pub async fn create_table(&self) -> Result<(), ApplicationError> {
        self.repo
            .create_table()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("create_table professores", e))
    }
}

#[async_trait]
impl ProfessorRepository for PostgresProfessorRepository {
    async fn save(&self, professor: &Professor) -> Result<Professor, ApplicationError> {
        let entity: ProfessorEntity = professor.clone().into();
        let saved = self
            .repo
            .insert(entity)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("insert professor", e))?;
        Ok(saved.into())
    }

    async fn find(&self, id: &str) -> Result<Option<Professor>, ApplicationError> {
        let found = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("find professor", e))?;
        Ok(found.map(|e| e.into()))
    }

    async fn list(&self) -> Result<Vec<Professor>, ApplicationError> {
        let items = self
            .repo
            .find_all()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("list professores", e))?;
        Ok(items.into_iter().map(|e| e.into()).collect())
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.repo
            .delete(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("delete professor", e))
    }
}

pub struct PostgresTurmaRepository {
    repo: PostgresRepository<TurmaEntity>,
}

impl PostgresTurmaRepository {
    pub fn new(connection: Arc<PostgresConnection>) -> Self {
        Self {
            repo: PostgresRepository::new(connection),
        }
    }

    pub async fn create_table(&self) -> Result<(), ApplicationError> {
        self.repo
            .create_table()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("create_table turmas", e))
    }
}

#[async_trait]
impl TurmaRepository for PostgresTurmaRepository {
    async fn save(&self, turma: &Turma) -> Result<Turma, ApplicationError> {
        let entity: TurmaEntity = turma.clone().into();
        let saved = self
            .repo
            .insert(entity)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("insert turma", e))?;
        Ok(saved.into())
    }

    async fn find(&self, id: &str) -> Result<Option<Turma>, ApplicationError> {
        let found = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("find turma", e))?;
        Ok(found.map(|e| e.into()))
    }

    async fn list(&self) -> Result<Vec<Turma>, ApplicationError> {
        let items = self
            .repo
            .find_all()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("list turmas", e))?;
        Ok(items.into_iter().map(|e| e.into()).collect())
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.repo
            .delete(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("delete turma", e))
    }
}

pub struct PostgresMatriculaRepository {
    repo: PostgresRepository<MatriculaEntity>,
}

impl PostgresMatriculaRepository {
    pub fn new(connection: Arc<PostgresConnection>) -> Self {
        Self {
            repo: PostgresRepository::new(connection),
        }
    }

    pub async fn create_table(&self) -> Result<(), ApplicationError> {
        self.repo
            .create_table()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("create_table matriculas", e))
    }
}

#[async_trait]
impl MatriculaRepository for PostgresMatriculaRepository {
    async fn save(&self, matricula: &Matricula) -> Result<Matricula, ApplicationError> {
        let entity: MatriculaEntity = matricula.clone().into();
        let saved = self
            .repo
            .insert(entity)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("insert matricula", e))?;
        Ok(saved.into())
    }

    async fn find(&self, id: &str) -> Result<Option<Matricula>, ApplicationError> {
        let found = self
            .repo
            .find_by_id(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("find matricula", e))?;
        Ok(found.map(|e| e.into()))
    }

    async fn list(&self) -> Result<Vec<Matricula>, ApplicationError> {
        let items = self
            .repo
            .find_all()
            .await
            .map_err(|e| ApplicationError::infrastructure_err("list matriculas", e))?;
        Ok(items.into_iter().map(|e| e.into()).collect())
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.repo
            .delete(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_err("delete matricula", e))
    }
}

pub struct PostgresAlunoPorTurmaQuery {
    connection: Arc<PostgresConnection>,
}

impl PostgresAlunoPorTurmaQuery {
    pub fn new(connection: Arc<PostgresConnection>) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl AlunoPorTurmaQuery for PostgresAlunoPorTurmaQuery {
    async fn listar_alunos_por_turma(
        &self,
        turma_id: &str,
    ) -> Result<Vec<Aluno>, ApplicationError> {
        let tabela = MatriculaEntity::collection_name();
        let query = format!("SELECT data FROM {} WHERE data->>'turma_id' = $1", tabela);
        let rows = sqlx::query(&query)
            .bind(turma_id)
            .fetch_all(self.connection.client())
            .await
            .map_err(|e| {
                ApplicationError::infrastructure_err("listar_alunos_por_turma query", e)
            })?;

        let aluno_repo = PostgresRepository::<AlunoEntity>::new(self.connection.clone());
        let mut alunos = Vec::new();
        let mut vistos = std::collections::HashSet::new();
        for row in rows {
            let data: String = row.get("data");
            let mat: MatriculaEntity = serde_json::from_str(&data)
                .map_err(|e| ApplicationError::infrastructure_err("deserialize matricula", e))?;
            if !vistos.insert(mat.aluno_id.clone()) {
                continue;
            }
            if let Some(entity) = aluno_repo
                .find_by_id(&mat.aluno_id)
                .await
                .map_err(|e| ApplicationError::infrastructure_err("find aluno in query", e))?
            {
                alunos.push(entity.into());
            }
        }
        Ok(alunos)
    }
}
