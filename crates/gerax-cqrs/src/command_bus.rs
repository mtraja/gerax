//! Porta de entrada para execução de Commands.

use std::sync::Arc;

use crate::command::Command;
use crate::error::CqrsError;
use crate::registry::HandlerRegistry;

/// Executa [`Command`]s tipadamente.
///
/// O usuário não precisa conhecer `Any`, `TypeId`, `ErasedHandler`
/// ou `HandlerAdapter`; esses são detalhes internos da infraestrutura.
#[derive(Clone)]
pub struct CommandBus {
    registry: Arc<HandlerRegistry>,
}

impl CommandBus {
    pub fn new(registry: Arc<HandlerRegistry>) -> Self {
        Self { registry }
    }

    /// Executa um `Command` e retorna o resultado tipado.
    pub async fn dispatch<C>(&self, command: C) -> Result<C::Output, CqrsError>
    where
        C: Command,
    {
        self.registry.dispatch_command(command).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AddOne;

    impl crate::Message for AddOne {
        type Output = i32;
    }

    impl Command for AddOne {}

    struct AddOneHandler;

    #[async_trait::async_trait]
    impl crate::CommandHandler for AddOneHandler {
        type Command = AddOne;

        async fn handle(&self, _command: AddOne) -> Result<i32, CqrsError> {
            Ok(42)
        }
    }

    #[tokio::test]
    async fn dispatch_retorna_output_tipado() {
        let mut registry = HandlerRegistry::new();
        registry.register_command(AddOneHandler).unwrap();

        let bus = CommandBus::new(Arc::new(registry));

        let value: i32 = bus.dispatch(AddOne).await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn dispatch_sem_handler_retorna_not_found() {
        let bus = CommandBus::new(Arc::new(HandlerRegistry::new()));

        let err = bus.dispatch(AddOne).await.unwrap_err();
        assert!(matches!(err, CqrsError::HandlerNotFound { .. }));
    }
}
