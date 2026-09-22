use std::sync::Arc;

use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::Repository;
use gerax_postgres::{PostgresConnection, PostgresRepository};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::application::errors::ApplicationError;
use crate::application::ports::outbound::TerapiaRepository;
use crate::domain::{
    Descricao, PacienteId, QtdSessoes, SessoesSemanais, Terapia, TerapiaId, TipoTerapia,
};

/// DTO de persistência de `Terapia` (JSONB no PostgreSQL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerapiaEntity {
    pub id: Option<String>,
    pub paciente_id: String,
    pub tipo: String,
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}

impl From<TerapiaEntity> for Terapia {
    fn from(value: TerapiaEntity) -> Self {
        Self {
            id: value.id.map(TerapiaId::from),
            paciente_id: PacienteId::from(value.paciente_id),
            tipo: TipoTerapia::from(value.tipo),
            descricao: Descricao::from(value.descricao),
            sessao_semanal: SessoesSemanais::from(value.sessao_semanal),
            qtd_sessoes: QtdSessoes::from(value.qtd_sessoes),
        }
    }
}

impl From<Terapia> for TerapiaEntity {
    fn from(value: Terapia) -> Self {
        Self {
            id: value.id.map(String::from),
            paciente_id: value.paciente_id.into(),
            tipo: value.tipo.into(),
            descricao: value.descricao.into(),
            sessao_semanal: value.sessao_semanal.into(),
            qtd_sessoes: value.qtd_sessoes.into(),
        }
    }
}

impl Entity for TerapiaEntity {
    fn collection_name() -> &'static str {
        "terapias"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

pub struct PostgresTerapiaRepository {
    repo: PostgresRepository<TerapiaEntity>,
}

impl PostgresTerapiaRepository {
    pub fn new(connection: Arc<PostgresConnection>) -> Self {
        Self {
            repo: PostgresRepository::new(connection),
        }
    }

    pub async fn create_table(&self) -> Result<(), ApplicationError> {
        self.repo
            .create_table()
            .await
            .map_err(ApplicationError::from)
    }
}

#[async_trait]
impl TerapiaRepository for PostgresTerapiaRepository {
    async fn create(&self, terapia: &Terapia) -> Result<Terapia, ApplicationError> {
        let entity: TerapiaEntity = terapia.clone().into();
        let saved = self
            .repo
            .insert(entity)
            .await
            .map_err(ApplicationError::from)?;
        Ok(saved.into())
    }

    async fn update(&self, terapia: &Terapia) -> Result<Terapia, ApplicationError> {
        let entity: TerapiaEntity = terapia.clone().into();
        self.repo
            .update(entity)
            .await
            .map_err(ApplicationError::from)?;
        Ok(terapia.clone())
    }

    async fn find(&self, id: &str) -> Result<Option<Terapia>, ApplicationError> {
        let found = self
            .repo
            .find_by_id(id)
            .await
            .map_err(ApplicationError::from)?;
        Ok(found.map(|e| e.into()))
    }

    async fn list(&self) -> Result<Vec<Terapia>, ApplicationError> {
        let items = self
            .repo
            .find_all()
            .await
            .map_err(ApplicationError::from)?;
        Ok(items.into_iter().map(|e| e.into()).collect())
    }

    async fn list_by_paciente(
        &self,
        paciente_id: &str,
    ) -> Result<Vec<Terapia>, ApplicationError> {
        let connection = self.repo.connection();
        let tabela = TerapiaEntity::collection_name();
        let query = format!(
            "SELECT data FROM {} WHERE data->>'paciente_id' = $1",
            tabela
        );

        let rows = sqlx::query(&query)
            .bind(paciente_id)
            .fetch_all(connection.client())
            .await
            .map_err(|e| ApplicationError::infrastructure_err("list_by_paciente query", e))?;

        let mut terapias = Vec::with_capacity(rows.len());
        for row in rows {
            let data: serde_json::Value = row.get("data");
            let entidade: TerapiaEntity = serde_json::from_value(data)
                .map_err(|e| ApplicationError::infrastructure_err("deserialize terapia", e))?;
            terapias.push(entidade.into());
        }
        Ok(terapias)
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.repo
            .delete(id)
            .await
            .map_err(ApplicationError::from)
    }
}