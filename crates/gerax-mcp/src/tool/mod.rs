//! Módulo de Tools.
//!
//! Uma Tool é uma função invocável exposta pelo servidor MCP, com
//! nome, descrição, schema de entrada e uma implementação assíncrona.

mod definition;
mod handler;
mod invocation;
mod registry;

pub use definition::{Tool, ToolError};
pub use handler::{FunctionTool, tool, tool_with};
pub use invocation::{CallToolResult, ImageContent, TextContent, ToolContent};
pub use registry::ToolRegistry;
