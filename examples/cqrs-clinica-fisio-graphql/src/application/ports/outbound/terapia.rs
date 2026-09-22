use async_trait::async_trait;

use crate::application::errors::ApplicationError;
use crate::domain::Terapia;

/// Porta de persistência de `Terapia`.
#[async_trait]
pub trait TerapiaRepository: Send + Sync {
    /// Persiste uma nova terapia e retorna a versão salva (com `id`).
    async fn create(&self, terapia: &Terapia) -> Result<Terapia, ApplicationError>;

    /// Atualiza uma terapia existente.
    async fn update(&self, terapia: &Terapia) -> Result<Terapia, ApplicationError>;

    /// Busca pelo id.
    async fn find(&self, id: &str) -> Result<Option<Terapia>, ApplicationError>;

    /// Lista todas.
    async fn list(&self) -> Result<Vec<Terapia>, ApplicationError>;

    /// Lista as terapias de um paciente.
    async fn list_by_paciente(&self, paciente_id: &str) -> Result<Vec<Terapia>, ApplicationError>;

    /// Remove pelo id.
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
}