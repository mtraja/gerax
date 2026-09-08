//! Composition root: registro de handlers e criação das buses.

use gerax_cqrs::{
    CommandBus, CqrsError, HandlerRegistry, QueryBus, register_commands, register_queries,
};

use crate::application::{AlunoRepository, CreateAlunoHandler, GetAlunoHandler};

/// Registra todos os handlers de `Aluno` no registry.
pub fn register_aluno(
    registry: &mut HandlerRegistry,
    repository: AlunoRepository,
) -> Result<(), CqrsError> {
    register_commands!(registry, CreateAlunoHandler::new(repository.clone()),);
    register_queries!(registry, GetAlunoHandler::new(repository.clone()),);
    Ok(())
}

/// Constrói as buses a partir de um registry compartilhado.
pub fn setup_buses(registry: std::sync::Arc<HandlerRegistry>) -> (CommandBus, QueryBus) {
    let command_bus = CommandBus::new(registry.clone());
    let query_bus = QueryBus::new(registry);
    (command_bus, query_bus)
}
