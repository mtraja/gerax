use std::sync::Arc;

use gerax_app::{DbError, PostgresConnection, PostgresRepository, Repository};
use gerax_core::Entity;
use sqlx::Row;

use crate::models::{Aluno, CriarMatricula, Matricula};

pub struct ServicoMatricula {
    db: Arc<PostgresConnection>,
}

impl ServicoMatricula {
    pub fn new(db: Arc<PostgresConnection>) -> Self {
        Self { db }
    }

    fn repo(&self) -> PostgresRepository<Matricula> {
        PostgresRepository::<Matricula>::new(self.db.clone())
    }

    pub async fn listar(&self) -> Result<Vec<Matricula>, DbError> {
        self.repo().find_all().await
    }

    pub async fn obter(&self, id: &str) -> Result<Option<Matricula>, DbError> {
        self.repo().find_by_id(id).await
    }

    pub async fn criar(&self, input: CriarMatricula) -> Result<Matricula, DbError> {
        let matricula = Matricula {
            id: None,
            aluno_id: input.aluno_id,
            turma_id: input.turma_id,
        };
        self.repo().insert(matricula).await
    }

    pub async fn deletar(&self, id: &str) -> Result<(), DbError> {
        self.repo().delete(id).await
    }

    pub async fn listar_alunos_por_turma(&self, turma_id: &str) -> Result<Vec<Aluno>, DbError> {
        let tabela = Matricula::collection_name();
        let query = format!("SELECT data FROM {} WHERE data->>'turma_id' = $1", tabela);
        let rows = sqlx::query(&query)
            .bind(turma_id)
            .fetch_all(self.db.client())
            .await
            .map_err(DbError::connection)?;

        let aluno_repo = PostgresRepository::<Aluno>::new(self.db.clone());
        let mut alunos = Vec::new();
        let mut vistos = std::collections::HashSet::new();
        for row in rows {
            let data: String = row.get("data");
            let mat: Matricula = serde_json::from_str(&data).map_err(DbError::serialization)?;
            if !vistos.insert(mat.aluno_id.clone()) {
                continue;
            }
            if let Some(aluno) = aluno_repo.find_by_id(&mat.aluno_id).await? {
                alunos.push(aluno);
            }
        }
        Ok(alunos)
    }
}
