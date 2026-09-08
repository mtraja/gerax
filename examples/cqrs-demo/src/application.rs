//! Camada de aplicação: handlers e ports (repository).
//!
//! Os handlers conhecem seu `Command`/`Query` explicitamente, mas os
//! Commands e Queries não conhecem seus handlers (baixo acoplamento).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use gerax_cqrs::{CommandHandler, CqrsError, QueryHandler};

use crate::domain::{Aluno, CreateAluno, GetAluno};

/// Port de persistência de `Aluno`.
#[derive(Debug, Default, Clone)]
pub struct AlunoRepository {
    dados: Arc<Mutex<HashMap<u64, Aluno>>>,
}

impl AlunoRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn save(&self, aluno: &Aluno) -> Result<(), CqrsError> {
        let mut dados = self
            .dados
            .lock()
            .map_err(|e| CqrsError::Execution(e.to_string()))?;
        dados.insert(aluno.id, aluno.clone());
        Ok(())
    }

    pub async fn find_by_id(&self, id: u64) -> Result<Option<Aluno>, CqrsError> {
        let dados = self
            .dados
            .lock()
            .map_err(|e| CqrsError::Execution(e.to_string()))?;
        Ok(dados.get(&id).cloned())
    }
}

pub struct CreateAlunoHandler {
    repository: AlunoRepository,
}

impl CreateAlunoHandler {
    pub fn new(repository: AlunoRepository) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl CommandHandler for CreateAlunoHandler {
    type Command = CreateAluno;

    async fn handle(&self, command: CreateAluno) -> Result<Aluno, CqrsError> {
        let aluno = Aluno {
            id: CreateAluno::novo_id(),
            nome: command.nome,
            email: command.email,
        };

        self.repository.save(&aluno).await?;

        Ok(aluno)
    }
}

pub struct GetAlunoHandler {
    repository: AlunoRepository,
}

impl GetAlunoHandler {
    pub fn new(repository: AlunoRepository) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl QueryHandler for GetAlunoHandler {
    type Query = GetAluno;

    async fn handle(&self, query: GetAluno) -> Result<Option<Aluno>, CqrsError> {
        self.repository.find_by_id(query.id).await
    }
}
