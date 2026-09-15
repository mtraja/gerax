//! [`McpServer`] e seu builder.
//!
//! O servidor é a porta de entrada da crate: agrega identificação,
//! capabilities, registries de Tools, Resources e Prompts, lifecycle
//! e o canal de notifications.
//!
//! ```
//! use gerax_mcp::McpServer;
//!
//! let server = McpServer::builder()
//!     .name("escola-cqrs")
//!     .version("0.1.0")
//!     .build();
//!
//! assert_eq!(server.name(), "escola-cqrs");
//! ```

use std::fmt;
use std::sync::{Arc, RwLock};

use tokio::sync::broadcast;

use crate::context::McpContext;
use crate::dispatcher::McpDispatcher;
use crate::error::McpError;
use crate::prompt::PromptRegistry;
use crate::protocol::{
    ImplementationInfo, JsonRpcMessage, JsonRpcNotification, ServerCapabilities,
    ServerPromptsCapability, ServerResourcesCapability, ServerToolsCapability,
};
use crate::resource::ResourceRegistry;
use crate::security::{McpAuthorizer, McpOperation};
use crate::tool::ToolRegistry;
use crate::transport::{StdioTransport, Transport};

/// Estado do lifecycle do servidor MCP.
///
/// ```text
/// Created → Initializing → Running
///    └─────────── shutdown ───────────┘
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpLifecycle {
    /// Servidor criado, aguardando `initialize`.
    #[default]
    Created,
    /// `initialize` recebido, falta `notifications/initialized`.
    Initializing,
    /// Ainda no handshake (compatibilidade futura com estados do spec).
    Initialized,
    /// Cliente confirmou inicialização; requests de operação aceitos.
    Running,
    /// Servidor em encerramento.
    Shutdown,
}

/// Servidor MCP.
#[derive(Clone)]
pub struct McpServer {
    name: String,
    version: String,
    instructions: Option<String>,
    explicit_capabilities: ServerCapabilities,
    tools: ToolRegistry,
    resources: ResourceRegistry,
    prompts: PromptRegistry,
    notifications_tx: broadcast::Sender<JsonRpcNotification>,
    lifecycle: Arc<RwLock<McpLifecycle>>,
    client_info: Arc<RwLock<Option<ImplementationInfo>>>,
    protocol_version: Arc<RwLock<Option<String>>>,
    authorizer: Option<Arc<dyn McpAuthorizer>>,
}

impl McpServer {
    /// Cria um builder para o servidor.
    pub fn builder() -> McpServerBuilder {
        McpServerBuilder::new()
    }

    /// Nome do servidor.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Versão do servidor.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Instruções de uso do servidor, quando definidas.
    pub fn instructions(&self) -> Option<&str> {
        self.instructions.as_deref()
    }

    /// Identificação da implementação do servidor.
    pub fn server_info(&self) -> ImplementationInfo {
        ImplementationInfo::new(self.name.clone(), self.version.clone())
    }

    /// Estado atual do lifecycle.
    pub fn lifecycle(&self) -> McpLifecycle {
        *self.lifecycle.read().expect("lifecycle poisoned")
    }

    /// Capabilities anunciadas, derivadas dos registries e mescladas
    /// com as capabilities explicitamente definidas no builder.
    pub fn capabilities(&self) -> ServerCapabilities {
        let mut capabilities = self.explicit_capabilities.clone();
        if !self.tools.names().is_empty() && capabilities.tools.is_none() {
            capabilities.tools = Some(ServerToolsCapability::default());
        }
        if !self.resources.uris().is_empty() && capabilities.resources.is_none() {
            capabilities.resources = Some(ServerResourcesCapability::default());
        }
        if !self.prompts.names().is_empty() && capabilities.prompts.is_none() {
            capabilities.prompts = Some(ServerPromptsCapability::default());
        }
        capabilities
    }

    /// Registro de Tools do servidor.
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Registro de Resources do servidor.
    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }

    /// Registro de Prompts do servidor.
    pub fn prompts(&self) -> &PromptRegistry {
        &self.prompts
    }

    /// Canal de notifications para o cliente.
    pub fn notifications(&self) -> NotificationSender {
        NotificationSender {
            sender: self.notifications_tx.clone(),
        }
    }

    /// Envia uma notification para o cliente.
    pub fn notify(&self, notification: JsonRpcNotification) -> Result<(), McpError> {
        self.notifications_tx
            .send(notification)
            .map(|_| ())
            .map_err(|_| McpError::Internal("falha ao enfileirar notification".to_owned()))
    }

    /// Notifica o cliente que a lista de Tools mudou.
    pub fn notify_tools_changed(&self) -> Result<(), McpError> {
        self.notify(crate::protocol::tools_list_changed_notification())
    }

    /// Notifica o cliente que a lista de Resources mudou.
    pub fn notify_resources_changed(&self) -> Result<(), McpError> {
        self.notify(crate::protocol::resources_list_changed_notification())
    }

    /// Notifica o cliente que a lista de Prompts mudou.
    pub fn notify_prompts_changed(&self) -> Result<(), McpError> {
        self.notify(crate::protocol::prompts_list_changed_notification())
    }

    /// Registra um comando do domínio como Tool MCP.
    ///
    /// A Tool usa [`C::NAME`](gerax_cqrs::CommandMetadata) como nome e
    /// argumentos JSON desserializados no comando `C`.
    #[cfg(feature = "cqrs")]
    pub fn register_command<C, B>(&self, command_bus: B) -> Result<(), McpError>
    where
        C: gerax_cqrs::Command + gerax_cqrs::CommandMetadata + serde::de::DeserializeOwned,
        C::Output: serde::Serialize,
        B: crate::cqrs::CommandDispatch<C> + 'static,
    {
        self.tools
            .register(crate::cqrs::CommandTool::<C, B>::from_metadata(command_bus))
            .map_err(McpError::from)
    }

    /// Inicia o loop principal do servidor sobre um transporte.
    ///
    /// Despacha requests, responde, e envia notifications até o fim da
    /// stream ou um erro de transporte.
    pub async fn run<T: Transport>(&self, transport: &mut T) -> Result<(), McpError> {
        let dispatcher = McpDispatcher::new(Arc::new(self.clone()));
        let mut notifications = self.notifications_tx.subscribe();

        loop {
            tokio::select! {
                received = transport.receive() => {
                    match received {
                        Ok(None) => return Ok(()),
                        Ok(Some(message)) => {
                            if let Some(response) = dispatcher.dispatch(message).await {
                                transport.send(JsonRpcMessage::Response(response)).await?;
                            }
                        }
                        Err(transport_error) => return Err(transport_error.into()),
                    }
                }
                notification = notifications.recv() => {
                    match notification {
                        Ok(notification) => transport.notify(notification).await?,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {}
                    }
                }
            }
        }
    }

    /// Inicia o servidor sobre stdin/stdout.
    pub async fn run_stdio(&self) -> Result<(), McpError> {
        let mut transport = StdioTransport::new();
        self.run(&mut transport).await
    }

    pub(crate) fn begin_initialize(&self) -> Result<(), McpError> {
        let mut lifecycle = self.lifecycle.write().expect("lifecycle poisoned");
        match *lifecycle {
            McpLifecycle::Created | McpLifecycle::Initializing => {
                *lifecycle = McpLifecycle::Initializing;
                Ok(())
            }
            _ => Err(McpError::InvalidRequest(
                "servidor já inicializado".to_owned(),
            )),
        }
    }

    pub(crate) fn complete_initialization(&self) -> Result<(), McpError> {
        let mut lifecycle = self.lifecycle.write().expect("lifecycle poisoned");
        match *lifecycle {
            McpLifecycle::Initializing | McpLifecycle::Initialized => {
                *lifecycle = McpLifecycle::Running;
                Ok(())
            }
            _ => Err(McpError::InvalidRequest(
                "notifications/initialized fora de ordem".to_owned(),
            )),
        }
    }

    pub(crate) fn require_running(&self) -> Result<(), McpError> {
        if self.lifecycle() == McpLifecycle::Running {
            Ok(())
        } else {
            Err(McpError::ServerNotInitialized)
        }
    }

    /// Encerra a sessão atual e volta ao estado inicial.
    ///
    /// Usado por transportes baseados em sessão (ex.: HTTP) para que um
    /// novo cliente possa iniciar um novo ciclo `initialize`.
    pub fn end_session(&self) {
        let mut lifecycle = self.lifecycle.write().expect("lifecycle poisoned");
        *lifecycle = McpLifecycle::Created;
        drop(lifecycle);
        *self.client_info.write().expect("client_info poisoned") = None;
        *self
            .protocol_version
            .write()
            .expect("protocol_version poisoned") = None;
    }

    pub(crate) fn set_client_info(&self, client_info: ImplementationInfo) {
        *self.client_info.write().expect("client_info poisoned") = Some(client_info);
    }

    pub(crate) fn set_protocol_version(&self, version: String) {
        *self
            .protocol_version
            .write()
            .expect("protocol_version poisoned") = Some(version);
    }

    pub(crate) fn client_info(&self) -> Option<ImplementationInfo> {
        self.client_info
            .read()
            .expect("client_info poisoned")
            .clone()
    }

    pub(crate) fn protocol_version(&self) -> Option<String> {
        self.protocol_version
            .read()
            .expect("protocol_version poisoned")
            .clone()
    }

    pub(crate) async fn authorize(
        &self,
        context: &McpContext,
        operation: &McpOperation,
    ) -> Result<(), McpError> {
        match &self.authorizer {
            Some(authorizer) => authorizer
                .authorize(context, operation)
                .await
                .map_err(McpError::from),
            None => Ok(()),
        }
    }
}

impl fmt::Debug for McpServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpServer")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("instructions", &self.instructions)
            .field("tools", &self.tools)
            .field("resources", &self.resources)
            .field("prompts", &self.prompts)
            .field("lifecycle", &self.lifecycle)
            .finish()
    }
}

/// Abstração sobre o canal de notifications do servidor.
#[derive(Debug, Clone)]
pub struct NotificationSender {
    sender: broadcast::Sender<JsonRpcNotification>,
}

impl NotificationSender {
    /// Subscreve ao canal de notifications.
    pub fn subscribe(&self) -> broadcast::Receiver<JsonRpcNotification> {
        self.sender.subscribe()
    }

    /// Envia uma notification, devolvendo o número de receptores.
    pub fn send(&self, notification: JsonRpcNotification) -> Result<usize, McpError> {
        self.sender
            .send(notification)
            .map_err(|_| McpError::Internal("falha ao enfileirar notification".to_owned()))
    }
}

/// Builder para [`McpServer`].
pub struct McpServerBuilder {
    name: String,
    version: String,
    instructions: Option<String>,
    explicit_capabilities: ServerCapabilities,
    tools: ToolRegistry,
    resources: ResourceRegistry,
    prompts: PromptRegistry,
    authorizer: Option<Arc<dyn McpAuthorizer>>,
}

impl McpServerBuilder {
    /// Cria um novo builder.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            instructions: None,
            explicit_capabilities: ServerCapabilities::default(),
            tools: ToolRegistry::new(),
            resources: ResourceRegistry::new(),
            prompts: PromptRegistry::new(),
            authorizer: None,
        }
    }

    /// Define o nome do servidor.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Define a versão do servidor.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Define as instruções de uso do servidor.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Define explicitamente as capabilities do servidor.
    ///
    /// As capabilities também são derivadas automaticamente dos
    /// registries (tools/resources/prompts registrados). Em caso de
    /// conflito, este valor tem precedência.
    pub fn capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.explicit_capabilities = capabilities;
        self
    }

    /// Registra uma Tool no servidor.
    ///
    /// Registros com nome inválido ou duplicado são silenciosamente
    /// ignorados; use [`McpServer::tools`] para diagnóstico.
    pub fn tool<T: crate::tool::Tool + 'static>(self, tool: T) -> Self {
        let _ = self.tools.register(tool);
        self
    }

    /// Registra um Resource no servidor.
    pub fn resource<T: crate::resource::Resource + 'static>(self, resource: T) -> Self {
        let _ = self.resources.register(Arc::new(resource));
        self
    }

    /// Registra um Prompt no servidor.
    pub fn prompt<P: crate::prompt::Prompt + 'static>(self, prompt: P) -> Self {
        let _ = self.prompts.register(prompt);
        self
    }

    /// Define o autorizador do servidor.
    pub fn authorizer<A: McpAuthorizer + 'static>(mut self, authorizer: A) -> Self {
        self.authorizer = Some(Arc::new(authorizer));
        self
    }

    /// Constrói o [`McpServer`].
    ///
    /// Nome vazio é substituído por `"gerax"` e versão vazia pela
    /// versão da crate.
    pub fn build(self) -> McpServer {
        McpServer {
            name: non_empty_or(self.name, "gerax"),
            version: non_empty_or(self.version, env!("CARGO_PKG_VERSION")),
            instructions: self.instructions,
            explicit_capabilities: self.explicit_capabilities,
            tools: self.tools,
            resources: self.resources,
            prompts: self.prompts,
            notifications_tx: broadcast::channel(64).0,
            lifecycle: Arc::new(RwLock::new(McpLifecycle::Created)),
            client_info: Arc::new(RwLock::new(None)),
            protocol_version: Arc::new(RwLock::new(None)),
            authorizer: self.authorizer,
        }
    }
}

impl Default for McpServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for McpServerBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpServerBuilder")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("instructions", &self.instructions)
            .finish()
    }
}

fn non_empty_or(value: String, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_owned()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::{Prompt, PromptContent, PromptError, PromptMessage};
    use crate::resource::Resource;
    use crate::tool::{CallToolResult, Tool, ToolError};
    use async_trait::async_trait;
    use serde_json::json;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn input_schema(&self) -> serde_json::Value {
            json!({ "type": "object" })
        }
        async fn call(&self, _args: serde_json::Value) -> Result<CallToolResult, ToolError> {
            Ok(CallToolResult::text("ok"))
        }
    }

    #[test]
    fn builder_builds_with_name_and_version() {
        let server = McpServer::builder()
            .name("escola-cqrs")
            .version("0.1.0")
            .build();

        assert_eq!(server.name(), "escola-cqrs");
        assert_eq!(server.version(), "0.1.0");
    }

    #[test]
    fn default_name_is_gerax() {
        let server = McpServer::builder().build();

        assert_eq!(server.name(), "gerax");
    }

    #[test]
    fn default_version_is_crate_version() {
        let server = McpServer::builder().build();

        assert_eq!(server.version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn empty_name_falls_back() {
        let server = McpServer::builder().name("").build();

        assert_eq!(server.name(), "gerax");
    }

    #[test]
    fn server_info_matches_name_and_version() {
        let server = McpServer::builder()
            .name("escola-cqrs")
            .version("0.1.0")
            .build();

        let info = server.server_info();

        assert_eq!(info.name, "escola-cqrs");
        assert_eq!(info.version, "0.1.0");
    }

    #[test]
    fn capabilities_default_is_empty() {
        let server = McpServer::builder().build();

        assert_eq!(server.capabilities(), ServerCapabilities::default());
    }

    #[test]
    fn capabilities_can_be_overridden() {
        let server = McpServer::builder()
            .capabilities(ServerCapabilities::default().with_tools())
            .build();

        assert!(server.capabilities().tools.is_some());
    }

    #[test]
    fn capabilities_are_derived_from_registries() {
        use crate::prompt::{GetPromptResult, PromptArgument};
        use crate::resource::{ResourceContents, ResourceError};

        struct DocResource;

        #[async_trait]
        impl Resource for DocResource {
            fn uri(&self) -> &str {
                "docs://manual"
            }
            fn name(&self) -> &str {
                "manual"
            }
            async fn read(&self) -> Result<ResourceContents, ResourceError> {
                Ok(ResourceContents::text("docs://manual", "conteúdo"))
            }
        }

        struct HelpPrompt;

        #[async_trait]
        impl Prompt for HelpPrompt {
            fn name(&self) -> &str {
                "help"
            }
            fn arguments(&self) -> Vec<PromptArgument> {
                Vec::new()
            }
            async fn get(&self, _args: serde_json::Value) -> Result<GetPromptResult, PromptError> {
                Ok(GetPromptResult::new(
                    None,
                    vec![PromptMessage::assistant(PromptContent::text("ajuda"))],
                ))
            }
        }

        let server = McpServer::builder()
            .tool(EchoTool)
            .resource(DocResource)
            .prompt(HelpPrompt)
            .build();

        let capabilities = server.capabilities();
        assert!(capabilities.tools.is_some());
        assert!(capabilities.resources.is_some());
        assert!(capabilities.prompts.is_some());
    }

    #[test]
    fn instructions_are_optional() {
        let without = McpServer::builder().build();
        let with = McpServer::builder().instructions("use with care").build();

        assert_eq!(without.instructions(), None);
        assert_eq!(with.instructions(), Some("use with care"));
    }

    #[test]
    fn lifecycle_starts_created() {
        let server = McpServer::builder().build();

        assert_eq!(server.lifecycle(), McpLifecycle::Created);
    }

    #[test]
    fn notifications_are_broadcast() {
        let server = McpServer::builder().build();
        let mut receiver = server.notifications().subscribe();

        server.notify_tools_changed().unwrap();

        let notification = receiver.blocking_recv().unwrap();
        assert_eq!(
            notification.method,
            crate::protocol::NOTIFICATIONS_TOOLS_LIST_CHANGED
        );
    }
}
