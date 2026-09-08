//! # cqrs-demo
//!
//! Exemplo de integração do `gerax-cqrs` com as macros derivadas.
//!
//! Demonstra o fluxo completo:
//!
//! ```text
//! CreateAluno → CreateAlunoHandler → CommandBus → Aluno
//! GetAluno    → GetAlunoHandler    → QueryBus   → Option<Aluno>
//! ```

mod application;
mod bootstrap;
mod domain;

use std::sync::Arc;

use gerax_cqrs::{CqrsError, HandlerRegistry};

use crate::application::AlunoRepository;
use crate::bootstrap::{register_aluno, setup_buses};
use crate::domain::{Aluno, CreateAluno, GetAluno};

#[tokio::main]
async fn main() -> Result<(), CqrsError> {
    let repository = AlunoRepository::new();

    let mut registry = HandlerRegistry::new();
    register_aluno(&mut registry, repository.clone())?;

    let registry = Arc::new(registry);
    let (command_bus, query_bus) = setup_buses(registry.clone());

    let aluno = command_bus
        .dispatch(CreateAluno {
            nome: "Marcos Trajano".into(),
            email: "marcos@email.com".into(),
        })
        .await?;

    println!("Created: {aluno:?}");

    let aluno_busca: Option<Aluno> = query_bus.execute(GetAluno { id: aluno.id }).await?;
    println!("Found:   {aluno_busca:?}");

    let aluno_inexistente = query_bus.execute(GetAluno { id: 999 }).await?;
    println!("Missing: {aluno_inexistente:?}");

    Ok(())
}
