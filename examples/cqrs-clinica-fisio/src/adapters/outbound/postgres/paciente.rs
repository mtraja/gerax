use std::sync::Arc;

use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::Repository;
use gerax_postgres::{PostgresConnection, PostgresRepository};
use serde::{Deserialize, Serialize};

use crate::application::errors::ApplicationError;
use crate::application::ports::outbound::PacienteRepository;
use crate::domain::{Cpf, DataNascimento, Email, Nome, Paciente, PacienteId, Telefone};

/// DTO de persistência de `Paciente` (JSONB no PostgreSQL).
///
/// Mantém o nome da tabela e o formato de serialização isolados no
/// adaptador, fora do domínio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacienteEntity {
    pub id: Option<String>,
    pub nome: String,
    pub cpf: String,
    pub data_nascimento: String,
    pub telefone: String,
    pub email: String,
}

impl From<PacienteEntity> for Paciente {
    fn from(value: PacienteEntity) -> Self {
        Self {
            id: value.id.map(PacienteId::from),
            nome: Nome::from(value.nome),
            cpf: Cpf::from(value.cpf),
            data_nascimento: DataNascimento::from(value.data_nascimento),
            telefone: Telefone::from(value.telefone),
            email: Email::from(value.email),
        }
    }
}

impl From<Paciente> for PacienteEntity {
    fn from(value: Paciente) -> Self {
        Self {
            id: value.id.map(String::from),
            nome: value.nome.into(),
            cpf: value.cpf.into(),
            data_nascimento: value.data_nascimento.into(),
            telefone: value.telefone.into(),
            email: value.email.into(),
        }
    }
}

impl Entity for PacienteEntity {
    fn collection_name() -> &'static str {
        "pacientes"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

pub struct PostgresPacienteRepository {
    repo: PostgresRepository<PacienteEntity>,
}

impl PostgresPacienteRepository {
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
impl PacienteRepository for PostgresPacienteRepository {
    async fn create(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError> {
        let entity: PacienteEntity = paciente.clone().into();
        let saved = self
            .repo
            .insert(entity)
            .await
            .map_err(ApplicationError::from)?;
        Ok(saved.into())
    }

    async fn update(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError> {
        let entity: PacienteEntity = paciente.clone().into();
        self.repo
            .update(entity)
            .await
            .map_err(ApplicationError::from)?;
        Ok(paciente.clone())
    }

    async fn find(&self, id: &str) -> Result<Option<Paciente>, ApplicationError> {
        let found = self
            .repo
            .find_by_id(id)
            .await
            .map_err(ApplicationError::from)?;
        Ok(found.map(|e| e.into()))
    }

    async fn list(&self) -> Result<Vec<Paciente>, ApplicationError> {
        let items = self
            .repo
            .find_all()
            .await
            .map_err(ApplicationError::from)?;
        Ok(items.into_iter().map(|e| e.into()).collect())
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.repo
            .delete(id)
            .await
            .map_err(ApplicationError::from)
    }
}