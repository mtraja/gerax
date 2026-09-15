//! Roteamento de mensagens JSON-RPC para o [`McpServer`].
//!
//! O [`McpDispatcher`] traduz mensagens do transporte em chamadas aos
//! registries e aplica o lifecycle e a autorização antes de responder.

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::context::McpContext;
use crate::error::McpError;
use crate::protocol::{
    CallToolRequestParams, GetPromptRequestParams, InitializeRequestParams, InitializeResult,
    JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ListPromptsResult,
    ListResourcesResult, ListToolsResult, MCP_PROTOCOL_VERSION, NOTIFICATIONS_INITIALIZED,
    PROMPTS_GET, PROMPTS_LIST, PromptMetadata, RESOURCES_LIST, RESOURCES_READ,
    ReadResourceRequestParams, ResourceMetadata, TOOLS_CALL, TOOLS_LIST, ToolMetadata,
};
use crate::security::McpOperation;
use crate::server::McpServer;

/// Despacha mensagens JSON-RPC para o servidor MCP.
#[derive(Debug)]
pub struct McpDispatcher {
    server: Arc<McpServer>,
}

impl McpDispatcher {
    /// Cria um dispatcher para o servidor.
    pub fn new(server: Arc<McpServer>) -> Self {
        Self { server }
    }

    /// Referência ao servidor.
    pub fn server(&self) -> &Arc<McpServer> {
        &self.server
    }

    /// Processa uma mensagem vinda do transporte.
    ///
    /// Requisições geram uma resposta; notifications e responses
    /// retornam `None`.
    pub async fn dispatch(&self, message: JsonRpcMessage) -> Option<JsonRpcResponse> {
        match message {
            JsonRpcMessage::Request(request) => {
                let response = self.handle_request(request).await;
                Some(response)
            }
            JsonRpcMessage::Notification(notification) => {
                self.handle_notification(notification).await;
                None
            }
            JsonRpcMessage::Response(_) => None,
        }
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result: Result<Value, McpError> = self.route(request.clone()).await;

        match result {
            Ok(result) => JsonRpcResponse::success(request.id, result),
            Err(error) => JsonRpcResponse::failure(request.id, error.to_jsonrpc_error()),
        }
    }

    async fn route(&self, request: JsonRpcRequest) -> Result<Value, McpError> {
        match request.method.as_str() {
            crate::protocol::initialize::INITIALIZE => {
                self.authorize(&request, McpOperation::Initialize).await?;
                self.handle_initialize(request).await
            }
            TOOLS_LIST => {
                self.authorize(&request, McpOperation::ListTools).await?;
                self.handle_tools_list(request).await
            }
            TOOLS_CALL => {
                let params: CallToolRequestParams = parse_params(&request)?;
                let name = params.name.clone();
                self.authorize(&request, McpOperation::CallTool(name))
                    .await?;
                self.handle_tools_call(request, params).await
            }
            RESOURCES_LIST => {
                self.authorize(&request, McpOperation::ListResources)
                    .await?;
                self.handle_resources_list(request).await
            }
            RESOURCES_READ => {
                let params: ReadResourceRequestParams = parse_params(&request)?;
                let uri = params.uri.clone();
                self.authorize(&request, McpOperation::ReadResource(uri))
                    .await?;
                self.handle_resources_read(request, params).await
            }
            PROMPTS_LIST => {
                self.authorize(&request, McpOperation::ListPrompts).await?;
                self.handle_prompts_list(request).await
            }
            PROMPTS_GET => {
                let params: GetPromptRequestParams = parse_params(&request)?;
                let name = params.name.clone();
                self.authorize(&request, McpOperation::GetPrompt(name))
                    .await?;
                self.handle_prompts_get(request, params).await
            }
            NOTIFICATIONS_INITIALIZED => Err(McpError::InvalidRequest(
                "notifications/initialized não deve ser enviada como request".to_owned(),
            )),
            other => Err(McpError::MethodNotFound(other.to_owned())),
        }
    }

    async fn handle_initialize(&self, request: JsonRpcRequest) -> Result<Value, McpError> {
        self.server.begin_initialize()?;

        let params: InitializeRequestParams = match request.params {
            Some(value) => serde_json::from_value(value)?,
            None => {
                return Err(McpError::InvalidRequest(
                    "initialize exige parâmetros".to_owned(),
                ));
            }
        };

        self.server.set_client_info(params.client_info);
        self.server
            .set_protocol_version(params.protocol_version.clone());

        let result = InitializeResult::new(
            MCP_PROTOCOL_VERSION,
            self.server.capabilities(),
            self.server.server_info(),
        );
        let result = match self.server.instructions() {
            Some(instructions) => result.with_instructions(instructions),
            None => result,
        };

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_tools_list(&self, request: JsonRpcRequest) -> Result<Value, McpError> {
        let _params: Option<Value> = request.params;
        self.server.require_running()?;

        let tools = self
            .server
            .tools()
            .list()
            .iter()
            .map(|tool| ToolMetadata::from_tool(tool.as_ref()))
            .collect::<Vec<_>>();

        Ok(serde_json::to_value(ListToolsResult::new(tools))?)
    }

    async fn handle_tools_call(
        &self,
        _request: JsonRpcRequest,
        params: CallToolRequestParams,
    ) -> Result<Value, McpError> {
        self.server.require_running()?;

        let arguments = params.arguments.unwrap_or_else(|| json!({}));
        let result = self
            .server
            .tools()
            .call(&params.name, arguments)
            .await
            .map_err(McpError::from)?;

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_resources_list(&self, request: JsonRpcRequest) -> Result<Value, McpError> {
        let _params: Option<Value> = request.params;
        self.server.require_running()?;

        let resources = self
            .server
            .resources()
            .list()
            .iter()
            .map(|resource| ResourceMetadata::from_resource(resource.as_ref()))
            .collect::<Vec<_>>();

        Ok(serde_json::to_value(ListResourcesResult::new(resources))?)
    }

    async fn handle_resources_read(
        &self,
        _request: JsonRpcRequest,
        params: ReadResourceRequestParams,
    ) -> Result<Value, McpError> {
        self.server.require_running()?;

        let contents = self
            .server
            .resources()
            .read(&params.uri)
            .await
            .map_err(McpError::from)?;

        Ok(serde_json::to_value(
            crate::protocol::ReadResourceResult::new(vec![contents]),
        )?)
    }

    async fn handle_prompts_list(&self, request: JsonRpcRequest) -> Result<Value, McpError> {
        let _params: Option<Value> = request.params;
        self.server.require_running()?;

        let prompts = self
            .server
            .prompts()
            .list()
            .iter()
            .map(|prompt| PromptMetadata::from_prompt(prompt.as_ref()))
            .collect::<Vec<_>>();

        Ok(serde_json::to_value(ListPromptsResult::new(prompts))?)
    }

    async fn handle_prompts_get(
        &self,
        _request: JsonRpcRequest,
        params: GetPromptRequestParams,
    ) -> Result<Value, McpError> {
        self.server.require_running()?;

        let arguments = params.arguments.unwrap_or_else(|| json!({}));
        let result = self
            .server
            .prompts()
            .execute(&params.name, arguments)
            .await
            .map_err(McpError::from)?;

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_notification(&self, notification: JsonRpcNotification) {
        if notification.method.as_str() == NOTIFICATIONS_INITIALIZED {
            let _ = self.server.complete_initialization();
        }
    }

    async fn authorize(
        &self,
        request: &JsonRpcRequest,
        operation: McpOperation,
    ) -> Result<(), McpError> {
        let context = McpContext::new(request.id.clone(), request.method.clone())
            .with_client_info(self.server.client_info())
            .with_protocol_version(self.server.protocol_version());

        self.server.authorize(&context, &operation).await
    }
}

fn parse_params<T: DeserializeOwned>(request: &JsonRpcRequest) -> Result<T, McpError> {
    let params = request.params.clone().unwrap_or(Value::Null);
    serde_json::from_value(params).map_err(McpError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::McpLifecycle;
    use async_trait::async_trait;

    fn params(protocol_version: &str) -> Value {
        json!({
            "protocolVersion": protocol_version,
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0" }
        })
    }

    fn init_request(id: i64) -> JsonRpcRequest {
        JsonRpcRequest::new(
            id,
            crate::protocol::initialize::INITIALIZE,
            Some(params(MCP_PROTOCOL_VERSION)),
        )
    }

    async fn init(dispatcher: &McpDispatcher, id: i64) -> JsonRpcResponse {
        dispatcher
            .dispatch(JsonRpcMessage::Request(init_request(id)))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn initialize_returns_result_and_lifecycle_progresses() {
        let server = Arc::new(McpServer::builder().build());
        let dispatcher = McpDispatcher::new(server.clone());

        let response = init(&dispatcher, 1).await;

        let result = response.result.as_ref().unwrap();
        assert_eq!(result["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], "gerax");
        assert_eq!(
            server.protocol_version().as_deref(),
            Some(MCP_PROTOCOL_VERSION)
        );

        // Ainda não "running" sem notifications/initialized.
        assert!(
            dispatcher
                .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                    2, TOOLS_LIST, None
                )))
                .await
                .unwrap()
                .error
                .is_some()
        );

        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        assert_eq!(server.lifecycle(), McpLifecycle::Running);
        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                3, TOOLS_LIST, None,
            )))
            .await
            .unwrap();
        assert_eq!(response.result, Some(json!({ "tools": [] })));
    }

    #[tokio::test]
    async fn unknown_method_gives_method_not_found() {
        let server = Arc::new(McpServer::builder().build());
        let dispatcher = McpDispatcher::new(server.clone());

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                1,
                "faz/coisa_estranha",
                None,
            )))
            .await
            .unwrap();

        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, -32601);
    }

    #[tokio::test]
    async fn requests_before_initialization_are_rejected() {
        let server = Arc::new(McpServer::builder().build());
        let dispatcher = McpDispatcher::new(server.clone());

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                5, TOOLS_LIST, None,
            )))
            .await
            .unwrap();

        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, -32000);
    }

    #[tokio::test]
    async fn tools_call_runs_and_returns_is_error() {
        let server = Arc::new(McpServer::builder().tool(FailingTool).build());
        let dispatcher = McpDispatcher::new(server.clone());

        init(&dispatcher, 1).await;
        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                2,
                TOOLS_CALL,
                Some(json!({ "name": "failing" })),
            )))
            .await
            .unwrap();

        let result = response.result.as_ref().unwrap();
        assert_eq!(result["isError"], true);
    }

    #[tokio::test]
    async fn tools_call_unknown_tool_is_invalid_params() {
        let server = Arc::new(McpServer::builder().build());
        let dispatcher = McpDispatcher::new(server.clone());

        init(&dispatcher, 1).await;
        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                2,
                TOOLS_CALL,
                Some(json!({ "name": "missing" })),
            )))
            .await
            .unwrap();

        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Unknown tool: missing");
    }

    #[tokio::test]
    async fn resources_read_returns_text() {
        let server = Arc::new(McpServer::builder().resource(StaticDoc).build());
        let dispatcher = McpDispatcher::new(server.clone());

        init(&dispatcher, 1).await;
        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                2,
                RESOURCES_READ,
                Some(json!({ "uri": "docs://manual" })),
            )))
            .await
            .unwrap();

        let result = response.result.as_ref().unwrap();
        assert_eq!(result["contents"][0]["text"], "conteúdo do manual");
    }

    #[tokio::test]
    async fn resources_read_missing_is_32002() {
        let server = Arc::new(McpServer::builder().build());
        let dispatcher = McpDispatcher::new(server.clone());

        init(&dispatcher, 1).await;
        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                2,
                RESOURCES_READ,
                Some(json!({ "uri": "docs://nao-existe" })),
            )))
            .await
            .unwrap();

        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, -32002);
    }

    #[tokio::test]
    async fn prompts_get_returns_messages() {
        let server = Arc::new(McpServer::builder().prompt(GreetingPrompt).build());
        let dispatcher = McpDispatcher::new(server.clone());

        init(&dispatcher, 1).await;
        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                2,
                PROMPTS_GET,
                Some(json!({ "name": "greeting", "arguments": { "name": "Ana" } })),
            )))
            .await
            .unwrap();

        let result = response.result.as_ref().unwrap();
        assert_eq!(result["messages"][0]["role"], "user");
        assert_eq!(result["messages"][0]["content"]["text"], "Olá, Ana!");
    }

    #[tokio::test]
    async fn prompts_get_unknown_prompt_is_invalid_params() {
        let server = Arc::new(McpServer::builder().build());
        let dispatcher = McpDispatcher::new(server.clone());

        init(&dispatcher, 1).await;
        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                2,
                PROMPTS_GET,
                Some(json!({ "name": "missing" })),
            )))
            .await
            .unwrap();

        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, -32602);
    }

    #[tokio::test]
    async fn authorizer_can_deny_tools_call() {
        struct DenyAll;
        #[async_trait]
        impl McpAuthorizer for DenyAll {
            async fn authorize(
                &self,
                _context: &McpContext,
                _operation: &McpOperation,
            ) -> Result<(), crate::security::AuthorizationError> {
                Err(crate::security::AuthorizationError::Denied("nope".into()))
            }
        }

        let server = Arc::new(
            McpServer::builder()
                .tool(EchoTestTool)
                .authorizer(DenyAll)
                .build(),
        );
        let dispatcher = McpDispatcher::new(server.clone());

        init(&dispatcher, 1).await;
        dispatcher
            .dispatch(JsonRpcMessage::Notification(
                crate::protocol::initialized_notification(),
            ))
            .await;

        let response = dispatcher
            .dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
                2, TOOLS_LIST, None,
            )))
            .await
            .unwrap();

        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, -32001);
    }

    // --- Tipos de teste ---

    use crate::prompt::{
        GetPromptResult, Prompt, PromptArgument, PromptContent, PromptError, PromptMessage,
    };
    use crate::resource::{Resource, ResourceContents, ResourceError};
    use crate::security::McpAuthorizer;
    use crate::tool::{CallToolResult, Tool, ToolError};

    struct EchoTestTool;

    #[async_trait]
    impl Tool for EchoTestTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn input_schema(&self) -> Value {
            json!({ "type": "object" })
        }
        async fn call(&self, _args: Value) -> Result<CallToolResult, ToolError> {
            Ok(CallToolResult::text("ok"))
        }
    }

    struct FailingTool;

    #[async_trait]
    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "failing"
        }
        fn input_schema(&self) -> Value {
            json!({ "type": "object" })
        }
        async fn call(&self, _args: Value) -> Result<CallToolResult, ToolError> {
            Ok(CallToolResult::text_error("falhou"))
        }
    }

    struct StaticDoc;

    #[async_trait]
    impl Resource for StaticDoc {
        fn uri(&self) -> &str {
            "docs://manual"
        }
        fn name(&self) -> &str {
            "manual"
        }
        async fn read(&self) -> Result<ResourceContents, ResourceError> {
            Ok(ResourceContents::text(
                "docs://manual",
                "conteúdo do manual",
            ))
        }
    }

    struct GreetingPrompt;

    #[async_trait]
    impl Prompt for GreetingPrompt {
        fn name(&self) -> &str {
            "greeting"
        }
        fn arguments(&self) -> Vec<PromptArgument> {
            vec![PromptArgument::new("name", Some("Nome"), true)]
        }
        async fn get(&self, arguments: Value) -> Result<GetPromptResult, PromptError> {
            let name = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PromptError::InvalidArgs {
                    name: "greeting".to_owned(),
                    reason: "argumento `name` ausente".to_owned(),
                })?;
            Ok(GetPromptResult::new(
                None,
                vec![PromptMessage::user(PromptContent::text(format!(
                    "Olá, {name}!"
                )))],
            ))
        }
    }
}
