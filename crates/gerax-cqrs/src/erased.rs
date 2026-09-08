//! Type erasure interna para armazenamento heterogêneo de handlers.
//!
//! Este módulo é detalhe interno da infraestrutura. O usuário da crate
//! não precisa (e não deve) trabalhar diretamente com estes tipos.

use std::any::Any;

use async_trait::async_trait;

use crate::error::CqrsError;
use crate::handler::{CommandHandler, QueryHandler};

/// Um handler apagado (erased).
///
/// Permite armazenar handlers heterogêneos no mesmo `HashMap`
/// utilizando `Box<dyn ErasedHandler>`.
#[async_trait]
pub(crate) trait ErasedHandler: Send + Sync {
    /// Executa o handler após fazer `downcast` da mensagem.
    async fn handle(&self, message: Box<dyn Any + Send>) -> Result<Box<dyn Any + Send>, CqrsError>;
}

/// Adapta um [`CommandHandler`] ao [`ErasedHandler`].
///
/// Recebe `Box<dyn Any + Send>`, faz `downcast` para `H::Command`,
/// executa `H::handle` e devolve o resultado em `Box<dyn Any + Send>`.
pub(crate) struct CommandHandlerAdapter<H> {
    pub(crate) handler: H,
}

#[async_trait]
impl<H> ErasedHandler for CommandHandlerAdapter<H>
where
    H: CommandHandler,
{
    async fn handle(&self, message: Box<dyn Any + Send>) -> Result<Box<dyn Any + Send>, CqrsError> {
        let command = message
            .downcast::<H::Command>()
            .map_err(|_| CqrsError::InvalidMessageType)?;

        let output = self.handler.handle(*command).await?;

        Ok(Box::new(output))
    }
}

/// Adapta um [`QueryHandler`] ao [`ErasedHandler`].
///
/// Recebe `Box<dyn Any + Send>`, faz `downcast` para `H::Query`,
/// executa `H::handle` e devolve o resultado em `Box<dyn Any + Send>`.
pub(crate) struct QueryHandlerAdapter<H> {
    pub(crate) handler: H,
}

#[async_trait]
impl<H> ErasedHandler for QueryHandlerAdapter<H>
where
    H: QueryHandler,
{
    async fn handle(&self, message: Box<dyn Any + Send>) -> Result<Box<dyn Any + Send>, CqrsError> {
        let query = message
            .downcast::<H::Query>()
            .map_err(|_| CqrsError::InvalidMessageType)?;

        let output = self.handler.handle(*query).await?;

        Ok(Box::new(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;

    struct AddOne;

    impl Message for AddOne {
        type Output = i32;
    }

    impl crate::Command for AddOne {}

    #[async_trait]
    impl CommandHandler for AddOne {
        type Command = AddOne;

        async fn handle(&self, _command: AddOne) -> Result<i32, CqrsError> {
            Ok(41)
        }
    }

    #[tokio::test]
    async fn adapter_downcasts_command_and_recovers_output() {
        let adapter = CommandHandlerAdapter { handler: AddOne };

        let message: Box<dyn Any + Send> = Box::new(AddOne);
        let output = adapter.handle(message).await.unwrap();

        let integer = *output.downcast::<i32>().expect("output should be i32");
        assert_eq!(integer, 41);
    }

    #[tokio::test]
    async fn adapter_rejects_wrong_message_type() {
        let adapter = CommandHandlerAdapter { handler: AddOne };

        let message: Box<dyn Any + Send> = Box::new(String::from("nope"));
        let err = adapter.handle(message).await.unwrap_err();

        assert!(matches!(err, CqrsError::InvalidMessageType));
    }
}
