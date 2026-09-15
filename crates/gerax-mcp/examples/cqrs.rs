//! Servidor MCP expondo um comando do domínio como Tool via
//! `gerax-cqrs` + [`McpServer::register_command`].
//!
//! Executar:
//!
//! ```bash
//! cargo run -p gerax-mcp --example cqrs
//! ```
//!
//! O comando `criar_aluno` fica disponível como `tools/call` MCP.

use std::sync::Arc;

use async_trait::async_trait;
use gerax_cqrs::{
    Command, CommandBus, CommandHandler, CommandMetadata, CqrsError, HandlerRegistry,
};
use gerax_mcp::McpServer;
use serde::Deserialize;

/// Comando do domínio.
#[derive(Debug, Deserialize)]
// O `email` faz parte do contrato de entrada; o handler de exemplo lê só o nome.
#[allow(dead_code)]
struct CriarAluno {
    nome: String,
    email: String,
}

/// Output do comando.
#[derive(Debug, serde::Serialize)]
struct Aluno {
    id: u64,
    nome: String,
}

impl gerax_cqrs::Message for CriarAluno {
    type Output = Aluno;
}

impl Command for CriarAluno {}

impl CommandMetadata for CriarAluno {
    const NAME: &'static str = "criar_aluno";
}

/// Handler do comando.
struct CriarAlunoHandler;

#[async_trait]
impl CommandHandler for CriarAlunoHandler {
    type Command = CriarAluno;

    async fn handle(&self, command: CriarAluno) -> Result<Aluno, CqrsError> {
        Ok(Aluno {
            id: 1,
            nome: command.nome,
        })
    }
}

fn command_bus() -> CommandBus {
    let mut registry = HandlerRegistry::new();
    registry.register_command(CriarAlunoHandler).unwrap();
    CommandBus::new(Arc::new(registry))
}

#[tokio::main]
async fn main() -> Result<(), gerax_mcp::McpError> {
    let server = McpServer::builder()
        .name("escola-cqrs")
        .version(env!("CARGO_PKG_VERSION"))
        .build();

    // Auto-registro: o comando vira Tool MCP com o nome `criar_aluno`.
    server.register_command::<CriarAluno, _>(command_bus())?;

    eprintln!(
        "[gerax-mcp] tool disponível: {}",
        server.tools().names().join(", ")
    );
    server.run_stdio().await
}
