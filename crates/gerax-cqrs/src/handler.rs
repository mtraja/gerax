//! Handlers de Commands e Queries.

use async_trait::async_trait;

use crate::command::Command;
use crate::error::CqrsError;
use crate::message::Message;
use crate::query::Query;

/// Executa um [`Command`].
///
/// O handler declara explicitamente qual [`Command`] executa através
/// do *associated type* [`CommandHandler::Command`]. Isso permite ao
/// [`crate::HandlerRegistry`] obter o `TypeId` correspondente sem que
/// o Command conheça o seu handler.
#[async_trait]
pub trait CommandHandler: Send + Sync + 'static {
    /// O tipo de [`Command`] tratado por este handler.
    type Command: Command;

    /// Executa a operação do `Command`.
    async fn handle(
        &self,
        command: Self::Command,
    ) -> Result<<Self::Command as Message>::Output, CqrsError>;
}

/// Executa uma [`Query`].
///
/// O handler declara explicitamente qual [`Query`] executa através
/// do *associated type* [`QueryHandler::Query`].
#[async_trait]
pub trait QueryHandler: Send + Sync + 'static {
    /// O tipo de [`Query`] tratada por este handler.
    type Query: Query;

    /// Executa a consulta.
    async fn handle(
        &self,
        query: Self::Query,
    ) -> Result<<Self::Query as Message>::Output, CqrsError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Aluno {
        id: u64,
        nome: String,
    }

    struct CreateAluno {
        nome: String,
    }

    impl Message for CreateAluno {
        type Output = Aluno;
    }

    impl Command for CreateAluno {}

    struct GetAluno {
        id: u64,
    }

    impl Message for GetAluno {
        type Output = Option<Aluno>;
    }

    impl Query for GetAluno {}

    struct CreateAlunoHandler;

    #[async_trait]
    impl CommandHandler for CreateAlunoHandler {
        type Command = CreateAluno;

        async fn handle(&self, command: CreateAluno) -> Result<Aluno, CqrsError> {
            Ok(Aluno {
                id: 1,
                nome: command.nome,
            })
        }
    }

    struct GetAlunoHandler;

    #[async_trait]
    impl QueryHandler for GetAlunoHandler {
        type Query = GetAluno;

        async fn handle(&self, query: GetAluno) -> Result<Option<Aluno>, CqrsError> {
            if query.id == 1 {
                Ok(Some(Aluno {
                    id: 1,
                    nome: "Marcos".into(),
                }))
            } else {
                Ok(None)
            }
        }
    }

    #[tokio::test]
    async fn command_handler_executes_and_returns_output() {
        let handler = CreateAlunoHandler;
        let aluno = handler
            .handle(CreateAluno {
                nome: "Marcos".into(),
            })
            .await
            .unwrap();

        assert_eq!(aluno.nome, "Marcos");
    }

    #[tokio::test]
    async fn query_handler_executes_and_returns_output() {
        let handler = GetAlunoHandler;
        let aluno = handler.handle(GetAluno { id: 1 }).await.unwrap();

        assert_eq!(aluno.unwrap().nome, "Marcos");
        assert!(handler.handle(GetAluno { id: 2 }).await.unwrap().is_none());
    }
}
