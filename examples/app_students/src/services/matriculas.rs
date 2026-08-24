use std::sync::Arc;

use gerax_app::{DbError, PostgresConnection, PostgresRepository, Repository};

use crate::models::{CriarMatricula, Matricula};

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
}
