//! # gerax-mcp
//!
//! Implementação do **Model Context Protocol (MCP)** para o Gerax.
//!
//! Permite que aplicações Gerax exponham **Tools**, **Resources** e
//! **Prompts** para agentes de IA através do protocolo MCP.
//!
//! ```text
//! MCP Protocol
//!      │
//!      ▼
//! JSON-RPC
//!      │
//!      ▼
//! McpDispatcher ← McpServer
//!      │
//!  ┌───┼─────────────┐
//!  ▼   ▼             ▼
//! Tools Resources   Prompts
//! ```

//! # Segurança e observabilidade
//!
//! A crate **nunca registra em logs** argumentos de Tools, conteúdos de
//! Resources, credenciais ou tokens: as implementações de `Debug` expõem
//! apenas estado de sessão e metadados, e [`McpClientError`] não inclui
//! dado sensível.
//!
//! A crate não se acopla a nenhum framework de telemetria. Uma integração
//! futura com `tracing` (opcional, via feature) poderá emitir spans
//! `mcp.request`, `mcp.response`, `mcp.tool.call`, `mcp.resource.read` e
//! `mcp.prompt.get` sem alterar a API pública.

#![warn(missing_docs)]

/// Camada de protocolo (JSON-RPC 2.0 e mensagens MCP).
pub mod protocol;

/// Despacho de mensagens JSON-RPC para o servidor.
pub mod dispatcher;

/// Transportes (STDIO, ...).
pub mod transport;

/// Cliente MCP (conecta a servidores e executa requests concorrentes).
pub mod client;

/// Servidor MCP.
pub mod server;

/// Tools invocáveis expostas pelo servidor.
pub mod tool;

/// Resources legíveis expostos pelo servidor.
pub mod resource;

/// Prompts (templates de mensagens) expostos pelo servidor.
pub mod prompt;

/// Contexto de request para autorização e extensões.
pub mod context;

/// Pontos de extensão de segurança.
pub mod security;

/// Erros da crate.
pub mod error;

/// Integração com `gerax-cqrs` (feature `cqrs`).
#[cfg(feature = "cqrs")]
pub mod cqrs;

pub use protocol::{
    CallToolRequestParams, GetPromptRequestParams, ImplementationInfo, InitializeRequestParams,
    InitializeResult, JSONRPC_VERSION, JsonRpcError, JsonRpcId, JsonRpcMessage,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ListPromptsParams, ListPromptsResult,
    ListResourcesParams, ListResourcesResult, ListToolsParams, ListToolsResult,
    MCP_PROTOCOL_VERSION, NOTIFICATIONS_INITIALIZED, NOTIFICATIONS_PROMPTS_LIST_CHANGED,
    NOTIFICATIONS_RESOURCES_LIST_CHANGED, NOTIFICATIONS_TOOLS_LIST_CHANGED, PromptMetadata,
    ReadResourceRequestParams, ReadResourceResult, ResourceMetadata, ResourceTemplateMetadata,
    ServerCapabilities, ServerPromptsCapability, ServerResourcesCapability, ServerToolsCapability,
    ToolMetadata, initialized_notification, jsonrpc, tools_list_changed_notification,
};

pub use server::{McpLifecycle, McpServer, McpServerBuilder, NotificationSender};

pub use resource::{
    BlobResourceContents, FailingResource, Resource, ResourceContents, ResourceError,
    ResourceRegistry, StaticResource, TextResourceContents,
};

pub use prompt::{
    GetPromptResult, Prompt, PromptArgument, PromptContent, PromptError, PromptImageContent,
    PromptMessage, PromptRegistry, PromptRole, PromptTextContent,
};

pub use tool::{
    CallToolResult, FunctionTool, ImageContent, TextContent, Tool, ToolContent, ToolError,
    ToolRegistry, tool, tool_with,
};

pub use context::McpContext;
pub use dispatcher::McpDispatcher;
pub use error::McpError;
pub use security::{AuthorizationError, McpAuthorizer, McpOperation};
pub use transport::{StdioTransport, Transport, TransportError};

pub use client::{
    McpClient, McpClientBuilder, McpClientError, McpSession, PromptDefinition, RemoteTool,
    RequestManager, ResourceDefinition, ResourceTemplateDefinition, ServerInfo, ToolCaller,
    ToolDefinition,
};

#[cfg(feature = "cqrs")]
pub use cqrs::{CommandDispatch, CommandTool, command_tool};
