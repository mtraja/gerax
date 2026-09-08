//! Registro de handlers heterogêneos por `TypeId`.

use std::any::TypeId;
use std::collections::HashMap;

use crate::command::Command;
use crate::erased::{CommandHandlerAdapter, ErasedHandler, QueryHandlerAdapter};
use crate::error::CqrsError;
use crate::handler::{CommandHandler, QueryHandler};
use crate::query::Query;

/// Registro de handlers de Commands e Queries.
///
/// Utiliza [`TypeId`] como chave primária para identificação em tempo
/// de execução e type erasure para armazenar handlers heterogêneos.
pub struct HandlerRegistry {
    command_handlers: HashMap<TypeId, Box<dyn ErasedHandler>>,
    query_handlers: HashMap<TypeId, Box<dyn ErasedHandler>>,
}

impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            command_handlers: HashMap::new(),
            query_handlers: HashMap::new(),
        }
    }

    /// Registra um [`CommandHandler`].
    ///
    /// O `TypeId` é obtido automaticamente de `H::Command`.
    /// Não substitui silenciosamente um handler já registrado.
    pub fn register_command<H>(&mut self, handler: H) -> Result<(), CqrsError>
    where
        H: CommandHandler,
    {
        let type_id = TypeId::of::<H::Command>();
        self.insert_command(type_id, handler)
    }

    /// Registra um [`QueryHandler`].
    ///
    /// O `TypeId` é obtido automaticamente de `H::Query`.
    /// Não substitui silenciosamente um handler já registrado.
    pub fn register_query<H>(&mut self, handler: H) -> Result<(), CqrsError>
    where
        H: QueryHandler,
    {
        let type_id = TypeId::of::<H::Query>();
        self.insert_query(type_id, handler)
    }

    /// Executa o handler de Commands para a mensagem dada.
    pub async fn dispatch_command<C>(&self, command: C) -> Result<C::Output, CqrsError>
    where
        C: Command,
    {
        let handler = self
            .command_handlers
            .get(&TypeId::of::<C>())
            .ok_or_else(|| CqrsError::HandlerNotFound {
                message: std::any::type_name::<C>(),
            })?;

        let output = handler.handle(Box::new(command)).await?;

        let output = *output
            .downcast::<C::Output>()
            .map_err(|_| CqrsError::InvalidOutputType)?;
        Ok(output)
    }

    /// Executa o handler de Queries para a mensagem dada.
    pub async fn dispatch_query<Q>(&self, query: Q) -> Result<Q::Output, CqrsError>
    where
        Q: Query,
    {
        let handler = self.query_handlers.get(&TypeId::of::<Q>()).ok_or_else(|| {
            CqrsError::HandlerNotFound {
                message: std::any::type_name::<Q>(),
            }
        })?;

        let output = handler.handle(Box::new(query)).await?;

        let output = *output
            .downcast::<Q::Output>()
            .map_err(|_| CqrsError::InvalidOutputType)?;
        Ok(output)
    }

    fn insert_command<H>(&mut self, type_id: TypeId, handler: H) -> Result<(), CqrsError>
    where
        H: CommandHandler,
    {
        if self.command_handlers.contains_key(&type_id) {
            return Err(CqrsError::HandlerAlreadyRegistered);
        }
        self.command_handlers
            .insert(type_id, Box::new(CommandHandlerAdapter { handler }));
        Ok(())
    }

    fn insert_query<H>(&mut self, type_id: TypeId, handler: H) -> Result<(), CqrsError>
    where
        H: QueryHandler,
    {
        if self.query_handlers.contains_key(&type_id) {
            return Err(CqrsError::HandlerAlreadyRegistered);
        }
        self.query_handlers
            .insert(type_id, Box::new(QueryHandlerAdapter { handler }));
        Ok(())
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;

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

    struct CreateAlunoHandler {
        next_id: u64,
    }

    #[async_trait::async_trait]
    impl CommandHandler for CreateAlunoHandler {
        type Command = CreateAluno;

        async fn handle(&self, command: CreateAluno) -> Result<Aluno, CqrsError> {
            Ok(Aluno {
                id: self.next_id,
                nome: command.nome,
            })
        }
    }

    struct GetAluno {
        id: u64,
    }

    impl Message for GetAluno {
        type Output = Option<Aluno>;
    }

    impl Query for GetAluno {}

    struct GetAlunoHandler;

    #[async_trait::async_trait]
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
    async fn registra_e_dispatcha_command() {
        let mut registry = HandlerRegistry::new();
        registry
            .register_command(CreateAlunoHandler { next_id: 7 })
            .unwrap();

        let aluno = registry
            .dispatch_command(CreateAluno { nome: "Ana".into() })
            .await
            .unwrap();

        assert_eq!(
            aluno,
            Aluno {
                id: 7,
                nome: "Ana".into()
            }
        );
    }

    #[tokio::test]
    async fn registra_e_dispatcha_query() {
        let mut registry = HandlerRegistry::new();
        registry.register_query(GetAlunoHandler).unwrap();

        let aluno = registry.dispatch_query(GetAluno { id: 1 }).await.unwrap();
        assert_eq!(aluno.unwrap().nome, "Marcos");

        let vazio = registry.dispatch_query(GetAluno { id: 9 }).await.unwrap();
        assert!(vazio.is_none());
    }

    #[tokio::test]
    async fn registro_duplicado_resulta_em_erro() {
        let mut registry = HandlerRegistry::new();
        registry
            .register_command(CreateAlunoHandler { next_id: 1 })
            .unwrap();

        let err = registry
            .register_command(CreateAlunoHandler { next_id: 2 })
            .unwrap_err();

        assert!(matches!(err, CqrsError::HandlerAlreadyRegistered));
    }

    #[tokio::test]
    async fn dispatch_sem_handler_retorna_not_found() {
        let registry = HandlerRegistry::new();

        let err = registry
            .dispatch_command(CreateAluno { nome: "X".into() })
            .await
            .unwrap_err();

        assert!(matches!(err, CqrsError::HandlerNotFound { .. }));
    }

    #[tokio::test]
    async fn registries_de_command_e_query_sao_independentes() {
        let mut registry = HandlerRegistry::new();
        registry
            .register_command(CreateAlunoHandler { next_id: 3 })
            .unwrap();
        registry.register_query(GetAlunoHandler).unwrap();

        let aluno = registry
            .dispatch_command(CreateAluno { nome: "Bia".into() })
            .await
            .unwrap();
        assert_eq!(aluno.id, 3);

        let consulta = registry.dispatch_query(GetAluno { id: 1 }).await.unwrap();
        assert!(consulta.is_some());
    }
}
