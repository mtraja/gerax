//! Integração com `gerax-cqrs`: Commands expostos como MCP Tools.
//!
//! O adapter [`CommandTool`] transforma um [`gerax_cqrs::Command`] de
//! `gerax-cqrs` em uma MCP [`Tool`], permitindo que agentes de IA
//! executem comandos de negócio através do `CommandBus` sem acoplar
//! regras de negócio ao protocolo MCP.
//!
//! ```text
//! AI Agent / LLM
//!    │
//!    ▼
//! MCP Tool
//!    │
//!    ▼
//! CommandDispatch
//!    │
//!    ▼
//! CommandBus
//!    │
//!    ▼
//! CommandHandler
//!    │
//!    ▼
//! Domain
//! ```
//!
//! Não duplica `CommandBus`, `CommandHandler` ou `CommandMetadata`:
//! todos são reutilizados de `gerax-cqrs`.

use std::marker::PhantomData;

use async_trait::async_trait;
use gerax_cqrs::{Command, CommandBus, CommandMetadata, CqrsError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::tool::{CallToolResult, Tool, ToolError};

/// Executa um [`gerax_cqrs::Command`] através de um bus de comandos.
///
/// Implementado pelo [`CommandBus`] de `gerax-cqrs` e extensível a
/// outras implementações de bus.
#[async_trait]
pub trait CommandDispatch<C: Command>: Send + Sync {
    /// Executa o comando e devolve o resultado tipado.
    async fn dispatch_command(&self, command: C) -> Result<C::Output, CqrsError>;
}

#[async_trait]
impl<C: Command> CommandDispatch<C> for CommandBus {
    async fn dispatch_command(&self, command: C) -> Result<C::Output, CqrsError> {
        self.dispatch(command).await
    }
}

/// Adapter que expõe um [`gerax_cqrs::Command`] de `gerax-cqrs` como MCP
/// [`Tool`].
///
/// Os argumentos JSON da Tool são desserializados no tipo do comando;
/// o resultado tipado é serializado como `structuredContent`.
pub struct CommandTool<C, B> {
    command_bus: B,
    name: String,
    description: Option<String>,
    input_schema: Value,
    _command: PhantomData<C>,
}

impl<C, B> CommandTool<C, B>
where
    C: Command + DeserializeOwned,
    C::Output: Serialize,
    B: CommandDispatch<C>,
{
    /// Cria uma Tool a partir de um comando e um bus.
    pub fn new(
        name: impl Into<String>,
        description: Option<impl Into<String>>,
        input_schema: Value,
        command_bus: B,
    ) -> Self {
        Self {
            command_bus,
            name: name.into(),
            description: description.map(Into::into),
            input_schema,
            _command: PhantomData,
        }
    }

    /// Cria uma Tool usando `C::NAME` de [`CommandMetadata`].
    ///
    /// O schema passa a ser um objeto aberto; refine com um builder
    /// quando o comando exigir validação de argumentos.
    pub fn from_metadata(command_bus: B) -> Self
    where
        C: CommandMetadata,
    {
        Self::new(
            C::NAME,
            None::<String>,
            json!({ "type": "object" }),
            command_bus,
        )
    }
}

#[async_trait]
impl<C, B> Tool for CommandTool<C, B>
where
    C: Command + DeserializeOwned,
    C::Output: Serialize,
    B: CommandDispatch<C>,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    async fn call(&self, arguments: Value) -> Result<CallToolResult, ToolError> {
        let command: C = serde_json::from_value(arguments).map_err(ToolError::Deserialize)?;
        let output = self
            .command_bus
            .dispatch_command(command)
            .await
            .map_err(|error| ToolError::Internal(error.to_string()))?;
        let value = serde_json::to_value(output).map_err(ToolError::Deserialize)?;

        Ok(CallToolResult::structured(value))
    }
}

/// Cria uma [`CommandTool`] para o comando dado.
///
/// Os argumentos JSON da Tool são desserializados no comando `C` e
/// despachados através do bus. O resultado tipado é serializado como
/// `structuredContent`.
///
/// **Exemplo:**
///
/// ```ignore
/// use gerax_mcp::{command_tool, CommandDispatch};
///
/// let tool = command_tool::<CreateAluno, _>(
///     "create_aluno",
///     "Cria um aluno",
///     serde_json::json!({ "type": "object" }),
///     command_bus,
/// );
/// ```
pub fn command_tool<C, B>(
    name: impl Into<String>,
    description: impl Into<String>,
    input_schema: Value,
    command_bus: B,
) -> CommandTool<C, B>
where
    C: Command + DeserializeOwned,
    C::Output: Serialize,
    B: CommandDispatch<C>,
{
    CommandTool::new(name, Some(description.into()), input_schema, command_bus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_cqrs::{CommandHandler, HandlerRegistry, Message};
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct CreateAluno {
        nome: String,
        email: String,
    }

    impl Message for CreateAluno {
        type Output = Aluno;
    }
    impl Command for CreateAluno {}
    impl CommandMetadata for CreateAluno {
        const NAME: &'static str = "create_aluno";
    }

    #[derive(Debug, Serialize, PartialEq)]
    struct Aluno {
        id: u64,
        nome: String,
    }

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

    fn bus() -> CommandBus {
        let mut registry = HandlerRegistry::new();
        registry.register_command(CreateAlunoHandler).unwrap();
        CommandBus::new(Arc::new(registry))
    }

    #[tokio::test]
    async fn command_exposed_as_mcp_tool() {
        let tool = CommandTool::<CreateAluno, _>::new(
            "create_aluno",
            Some("Cria um aluno"),
            json!({
                "type": "object",
                "properties": {
                    "nome": { "type": "string" },
                    "email": { "type": "string" }
                },
                "required": ["nome", "email"]
            }),
            bus(),
        );

        assert_eq!(tool.name(), "create_aluno");
        assert_eq!(tool.description(), Some("Cria um aluno"));

        let result = tool
            .call(json!({ "nome": "Marcos", "email": "m@x.com" }))
            .await
            .unwrap();

        assert_eq!(
            result.structured_content,
            Some(json!({ "id": 1, "nome": "Marcos" }))
        );
    }

    #[tokio::test]
    async fn invalid_arguments_are_rejected() {
        let tool = CommandTool::<CreateAluno, _>::new(
            "create_aluno",
            None::<String>,
            json!({ "type": "object" }),
            bus(),
        );

        let error = tool.call(json!({})).await.unwrap_err();

        assert!(matches!(error, ToolError::Deserialize(_)));
    }

    #[tokio::test]
    async fn from_metadata_uses_command_name() {
        let tool = CommandTool::<CreateAluno, _>::from_metadata(bus());

        assert_eq!(tool.name(), "create_aluno");
    }

    #[tokio::test]
    async fn command_tool_free_function() {
        let tool = command_tool::<CreateAluno, _>(
            "criar_aluno",
            "Cria um aluno",
            json!({ "type": "object" }),
            bus(),
        );

        assert_eq!(tool.name(), "criar_aluno");

        let result = tool
            .call(json!({ "nome": "Ana", "email": "a@x.com" }))
            .await
            .unwrap();
        assert_eq!(
            result.structured_content,
            Some(json!({ "id": 1, "nome": "Ana" }))
        );
    }
}
