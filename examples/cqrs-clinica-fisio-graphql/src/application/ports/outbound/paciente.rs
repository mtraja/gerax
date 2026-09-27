use async_trait::async_trait;

use crate::application::errors::ApplicationError;
use crate::domain::Paciente;

/// Porta de persistência de `Paciente`.
#[async_trait]
pub trait PacienteRepository: Send + Sync {
    /// Persiste um novo paciente e retorna a versão salva (com `id`).
    async fn create(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError>;

    /// Atualiza um paciente existente.
    async fn update(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError>;

    /// Busca pelo id.
    async fn find(&self, id: &str) -> Result<Option<Paciente>, ApplicationError>;

    /// Lista todos.
    async fn list(&self) -> Result<Vec<Paciente>, ApplicationError>;

    /// Remove pelo id.
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
}
