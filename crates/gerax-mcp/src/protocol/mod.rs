//! Camada de protocolo do Model Context Protocol.
//!
//! Contém os tipos JSON-RPC 2.0 e as mensagens MCP de inicialização,
//! capabilities, notifications, tools, resources e prompts.

pub mod capabilities;
pub mod initialize;
pub mod jsonrpc;
pub mod notifications;
pub mod prompts;
pub mod resources;
pub mod tools;

pub use capabilities::{
    ClientCapabilities, ClientRootsCapability, ServerCapabilities, ServerPromptsCapability,
    ServerResourcesCapability, ServerToolsCapability,
};
pub use initialize::{
    ImplementationInfo, InitializeRequestParams, InitializeResult, MCP_PROTOCOL_VERSION,
};
pub use jsonrpc::{
    JSONRPC_VERSION, JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse,
};
pub use notifications::{
    NOTIFICATIONS_INITIALIZED, NOTIFICATIONS_PROMPTS_LIST_CHANGED,
    NOTIFICATIONS_RESOURCES_LIST_CHANGED, NOTIFICATIONS_TOOLS_LIST_CHANGED,
    initialized_notification, message_notification, prompts_list_changed_notification,
    resources_list_changed_notification, tools_list_changed_notification,
};
pub use prompts::{
    GetPromptRequestParams, GetPromptResultJson, ListPromptsParams, ListPromptsResult, PROMPTS_GET,
    PROMPTS_LIST, PromptMetadata,
};
pub use resources::{
    ListResourceTemplatesParams, ListResourceTemplatesResult, ListResourcesParams,
    ListResourcesResult, RESOURCES_LIST, RESOURCES_READ, RESOURCES_TEMPLATES_LIST,
    ReadResourceRequestParams, ReadResourceResult, ResourceMetadata, ResourceTemplateMetadata,
};
pub use tools::{
    CallToolRequestParams, CallToolResultJson, ListToolsParams, ListToolsResult, TOOLS_CALL,
    TOOLS_LIST, ToolMetadata,
};
