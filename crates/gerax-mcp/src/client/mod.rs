//! Cliente MCP.
//!
//! Conecta a servidores MCP, negocia uma sessão (`initialize`) e executa
//! requests concorrentes com correlação correta de respostas, de forma
//! independente do transporte concreto.
//!
//! # Uso
//!
//! O fluxo típico é construído com o builder e um [`Transport`]:
//!
//! ```no_run
//! use std::error::Error;
//! use gerax_mcp::{McpClient, StdioTransport};
//! use serde_json::json;
//!
//! # async fn run() -> Result<(), Box<dyn Error>> {
//! let client = McpClient::<StdioTransport>::builder()
//!     .name("gerax-agent")
//!     .version("0.1.0")
//!     .connect(StdioTransport::command("my-mcp-server").build().await?)
//!     .await?;
//!
//! client.initialize().await?;
//!
//! let tools = client.list_tools().await?;
//! println!("{} tools disponíveis", tools.len());
//!
//! let result = client
//!     .call_tool("search", json!({ "query": "Rust" }))
//!     .await?;
//!
//! client.close().await?;
//! # Ok(())
//! # }
//! ```

pub mod capabilities;
pub mod definitions;
mod dispatcher;
pub mod error;
mod mcp_client;
pub mod remote_tool;
pub mod request_manager;
pub mod session;

/// Transporte de teste, disponível em testes e com a feature `test-utils`.
#[cfg(any(test, feature = "test-utils"))]
pub mod mock;

pub use definitions::{
    PromptDefinition, ResourceDefinition, ResourceTemplateDefinition, ToolDefinition,
};
pub use error::McpClientError;
pub use mcp_client::{McpClient, McpClientBuilder};
pub use remote_tool::{RemoteTool, ToolCaller};
pub use request_manager::RequestManager;
pub use session::{McpSession, ServerInfo};

/// Mensagens de saída do cliente (requests e notifications), compartilhadas
/// entre o [`RequestManager`] e o loop do transporte.
pub(crate) use mcp_client::OutboundMessage;

/// Transporte de teste, disponível em testes e com a feature `test-utils`.
#[cfg(any(test, feature = "test-utils"))]
pub use mock::{MockServerHandle, MockTransport};
