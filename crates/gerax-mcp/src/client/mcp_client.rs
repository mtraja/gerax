//! [`McpClient`]: cliente MCP desacoplado do transporte.
//!
//! O cliente conecta a um servidor MCP, executa o handshake `initialize`,
//! envia requests concorrentes e correlaciona as responses pelos IDs
//! JSON-RPC, independentemente do transporte (STDIO, HTTP, ...).
//!
//! ```text
//! McpClient<T>
//!      │
//!      ▼
//! RequestManager
//!      │
//!      ▼
//!   Transport
//! ```

use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Mutex as SyncMutex;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};

use crate::prompt::GetPromptResult;
use crate::protocol::{
    CallToolRequestParams, ClientCapabilities, GetPromptRequestParams, ImplementationInfo,
    InitializeRequestParams, InitializeResult, JsonRpcId, JsonRpcMessage, JsonRpcNotification,
    JsonRpcRequest, JsonRpcResponse, ListPromptsParams, ListPromptsResult,
    ListResourceTemplatesParams, ListResourceTemplatesResult, ListResourcesParams,
    ListResourcesResult, ListToolsParams, ListToolsResult, MCP_PROTOCOL_VERSION,
    NOTIFICATIONS_PROMPTS_LIST_CHANGED, NOTIFICATIONS_RESOURCES_LIST_CHANGED,
    NOTIFICATIONS_TOOLS_LIST_CHANGED, PROMPTS_GET, PROMPTS_LIST, RESOURCES_LIST, RESOURCES_READ,
    RESOURCES_TEMPLATES_LIST, ReadResourceRequestParams, ReadResourceResult, TOOLS_CALL,
    TOOLS_LIST,
};
use crate::tool::CallToolResult;
use crate::transport::{Transport, TransportError};

use super::definitions::{
    PromptDefinition, ResourceDefinition, ResourceTemplateDefinition, ToolDefinition,
};
use super::dispatcher::ResponseDispatcher;
use super::error::McpClientError;
use super::remote_tool::{RemoteTool, ToolCaller};
use super::request_manager::RequestManager;
use super::session::McpSession;

/// Estado do ciclo de vida do cliente.
///
/// ```text
/// Connected → Initializing → Initialized → Closed
/// ```
///
/// Reconexão automática **não** é implementada nesta versão: após
/// [`Closed`](ClientState::Closed), crie um novo cliente. A máquina de
/// estados, porém, preserva espaço para um futuro
/// `Disconnected → Reconnect → Initialize → Initialized` sem expor
/// reconexão escondida dentro do [`McpClient`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientState {
    /// Conectado ao transporte, mas ainda sem handshake.
    Connected,
    /// `initialize` enviado, aguardando a resposta.
    Initializing,
    /// Handshake concluído; operações de sessão liberadas.
    Initialized,
    /// Cliente fechado.
    Closed,
}

/// Configuração interna do cliente.
#[derive(Debug, Clone)]
pub(crate) struct ClientConfig {
    pub name: String,
    pub version: String,
    pub capabilities: ClientCapabilities,
    pub request_timeout: Duration,
}

/// Cliente MCP independente do transporte concreto.
///
/// O transporte é entremeado no tipo (`T: Transport`) e é consumido
/// por uma tarefa interna de leitura/escrita. Instâncias são
/// `Send + Sync` (independentemente de `T`), podendo ser
/// compartilhadas entre tarefas via [`Arc`].
///
/// ```ignore
/// use std::sync::Arc;
/// use gerax_mcp::McpClient;
///
/// let client = Arc::new(McpClient::builder()
///     .name("gerax-agent")
///     .version("0.1.0")
///     .connect(transport) // transport: qualquer T: Transport
///     .await?);
/// let client = client.clone();
/// tokio::spawn(async move { client.initialize().await });
/// ```
pub struct McpClient<T> {
    request_manager: Arc<RequestManager>,
    state: RwLock<ClientState>,
    session: RwLock<Option<McpSession>>,
    config: Arc<ClientConfig>,
    notifications_tx: broadcast::Sender<JsonRpcNotification>,
    discovery: Arc<DiscoveryCache>,
    handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    _transport: PhantomData<fn() -> T>,
}

impl<T> McpClient<T> {
    /// Cria um builder para o cliente MCP.
    pub fn builder() -> McpClientBuilder {
        McpClientBuilder::new()
    }
}

impl<T: Transport> McpClient<T> {
    /// Executa o handshake MCP `initialize`.
    ///
    /// Envia `protocolVersion`, `capabilities` e `clientInfo`, armazena
    /// a sessão negociada e, em seguida, envia a notification
    /// `notifications/initialized`. Operações de sessão ficam liberadas
    /// somente após esta chamada.
    pub async fn initialize(&self) -> Result<McpSession, McpClientError> {
        {
            let mut state = self.state.write().await;
            match *state {
                ClientState::Initialized | ClientState::Initializing => {
                    return Err(McpClientError::AlreadyInitialized);
                }
                ClientState::Closed => return Err(McpClientError::Closed),
                ClientState::Connected => {}
            }
            *state = ClientState::Initializing;
        }

        let params = InitializeRequestParams::new(
            MCP_PROTOCOL_VERSION,
            self.config.capabilities.clone(),
            ImplementationInfo::new(self.config.name.clone(), self.config.version.clone()),
        );
        let request = JsonRpcRequest::new(
            self.next_id(),
            crate::protocol::initialize::INITIALIZE,
            Some(serde_json::to_value(params)?),
        );

        let response = match self
            .request_manager
            .send_request(request, self.config.request_timeout)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                self.reset_initialization().await;
                return Err(error);
            }
        };

        let payload = match payload_from_response(crate::protocol::initialize::INITIALIZE, response)
        {
            Ok(payload) => payload,
            Err(error) => {
                self.reset_initialization().await;
                return Err(error);
            }
        };

        let result: InitializeResult = match serde_json::from_value(payload) {
            Ok(result) => result,
            Err(error) => {
                self.reset_initialization().await;
                return Err(McpClientError::Deserialize(error));
            }
        };

        let session = McpSession::new(
            result.protocol_version,
            result.server_info,
            result.capabilities,
        );

        // Notification `notifications/initialized`: o servidor libera
        // operações de sessão a partir deste ponto.
        self.request_manager
            .send_notification(crate::protocol::initialized_notification())
            .await?;

        *self.session.write().await = Some(session.clone());
        *self.state.write().await = ClientState::Initialized;
        Ok(session)
    }

    /// Envia um request JSON-RPC arbitrário e devolve o resultado bruto.
    ///
    /// Exige que `initialize` já tenha sido concluído; caso contrário,
    /// retorna [`McpClientError::NotInitialized`].
    pub async fn request(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, McpClientError> {
        self.require_initialized().await?;
        let request = JsonRpcRequest::new(self.next_id(), method, params);
        let response = self
            .request_manager
            .send_request(request, self.config.request_timeout)
            .await?;
        payload_from_response(method, response)
    }

    /// Sessão negociada no `initialize`, se concluída.
    pub async fn session(&self) -> Option<McpSession> {
        self.session.read().await.clone()
    }

    /// Lista todas as Tools expostas pelo servidor, atravessando as
    /// páginas de `tools/list` quando necessário.
    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>, McpClientError> {
        let session = self.session_snapshot().await?;
        if !session.supports_tools() {
            return Err(McpClientError::Unsupported(TOOLS_LIST.to_owned()));
        }
        self.paginate(|cursor| async move { self.list_tools_page(cursor).await })
            .await
    }

    /// Tools do servidor com cache local.
    ///
    /// Na primeira chamada, percorre `tools/list` e guarda o resultado;
    /// chamadas seguintes devolvem a cópia em cache sem tocar no
    /// servidor. A cache é invalidada automaticamente pela notification
    /// `notifications/tools/list_changed`. Para força uma busca nova,
    /// use [`list_tools`](Self::list_tools).
    pub async fn tools(&self) -> Result<Vec<ToolDefinition>, McpClientError> {
        if let Some(cached) = self.discovery.tools() {
            return Ok(cached);
        }
        let tools = self.list_tools().await?;
        *self.discovery.tools.lock().unwrap() = Some(tools.clone());
        Ok(tools)
    }

    /// Uma página de `tools/list`.
    ///
    /// Retorna o resultado do protocolo, incluindo o `nextCursor` para
    /// a próxima página. Use [`list_tools`](Self::list_tools) para
    /// coletar todas as páginas.
    pub async fn list_tools_page(
        &self,
        cursor: Option<String>,
    ) -> Result<(Vec<ToolDefinition>, Option<String>), McpClientError> {
        let value = self
            .request_typed(TOOLS_LIST, ListToolsParams { cursor })
            .await?;
        let result: ListToolsResult =
            serde_json::from_value(value).map_err(McpClientError::Deserialize)?;
        Ok((
            result.tools.into_iter().map(ToolDefinition::from).collect(),
            result.next_cursor,
        ))
    }

    /// Invoca uma Tool com os argumentos fornecidos.
    pub async fn call_tool(
        &self,
        name: impl Into<String>,
        arguments: Value,
    ) -> Result<CallToolResult, McpClientError> {
        let session = self.session_snapshot().await?;
        if !session.supports_tools() {
            return Err(McpClientError::Unsupported(TOOLS_CALL.to_owned()));
        }
        let payload = self
            .request_typed(
                TOOLS_CALL,
                CallToolRequestParams::new(name, Some(arguments)),
            )
            .await?;
        serde_json::from_value(payload).map_err(McpClientError::Deserialize)
    }

    /// Descobre todas as Tools remotas, prontas para invocação direta.
    ///
    /// ```no_run
    /// # async fn run(client: gerax_mcp::McpClient<gerax_mcp::StdioTransport>) {
    /// let tools = client.remote_tools().await.unwrap();
    /// for tool in &tools {
    ///     println!("{}", tool.name());
    /// }
    /// # }
    /// ```
    pub async fn remote_tools(&self) -> Result<Vec<RemoteTool>, McpClientError> {
        let definitions = self.list_tools().await?;
        let caller: Arc<dyn ToolCaller> = Arc::new(ClientToolCaller {
            request_manager: self.request_manager.clone(),
            config: self.config.clone(),
        });
        Ok(definitions
            .into_iter()
            .map(|definition| RemoteTool::new(definition, caller.clone()))
            .collect())
    }

    /// Lista todos os Resources expostos pelo servidor, atravessando
    /// as páginas de `resources/list` quando necessário.
    pub async fn list_resources(&self) -> Result<Vec<ResourceDefinition>, McpClientError> {
        let session = self.session_snapshot().await?;
        if !session.supports_resources() {
            return Err(McpClientError::Unsupported(RESOURCES_LIST.to_owned()));
        }
        self.paginate(|cursor| async move { self.list_resources_page(cursor).await })
            .await
    }

    /// Resources do servidor com cache local.
    ///
    /// Comportamento análogo a [`tools`](Self::tools): a cache é
    /// preenchida na primeira chamada e invalidada pela notification
    /// `notifications/resources/list_changed`.
    pub async fn resources(&self) -> Result<Vec<ResourceDefinition>, McpClientError> {
        if let Some(cached) = self.discovery.resources() {
            return Ok(cached);
        }
        let resources = self.list_resources().await?;
        *self.discovery.resources.lock().unwrap() = Some(resources.clone());
        Ok(resources)
    }

    /// Uma página de `resources/list`.
    pub async fn list_resources_page(
        &self,
        cursor: Option<String>,
    ) -> Result<(Vec<ResourceDefinition>, Option<String>), McpClientError> {
        let value = self
            .request_typed(RESOURCES_LIST, ListResourcesParams { cursor })
            .await?;
        let result: ListResourcesResult =
            serde_json::from_value(value).map_err(McpClientError::Deserialize)?;
        Ok((
            result
                .resources
                .into_iter()
                .map(ResourceDefinition::from)
                .collect(),
            result.next_cursor,
        ))
    }

    /// Lê o conteúdo de um Resource pelo seu URI.
    pub async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpClientError> {
        let session = self.session_snapshot().await?;
        if !session.supports_resources() {
            return Err(McpClientError::Unsupported(RESOURCES_READ.to_owned()));
        }
        let params = ReadResourceRequestParams {
            uri: uri.to_owned(),
        };
        let payload = self.request_typed(RESOURCES_READ, params).await?;
        serde_json::from_value(payload).map_err(McpClientError::Deserialize)
    }

    /// Lista todos os Resource Templates do servidor, atravessando as
    /// páginas de `resources/templates/list`.
    ///
    /// Mantida separada de [`list_resources`](Self::list_resources):
    /// templates são metadados de *famílias* de resources, não resources
    /// concretos.
    pub async fn list_resource_templates(
        &self,
    ) -> Result<Vec<ResourceTemplateDefinition>, McpClientError> {
        let session = self.session_snapshot().await?;
        if !session.supports_resources() {
            return Err(McpClientError::Unsupported(
                RESOURCES_TEMPLATES_LIST.to_owned(),
            ));
        }
        self.paginate(|cursor| async move { self.list_resource_templates_page(cursor).await })
            .await
    }

    /// Uma página de `resources/templates/list`.
    pub async fn list_resource_templates_page(
        &self,
        cursor: Option<String>,
    ) -> Result<(Vec<ResourceTemplateDefinition>, Option<String>), McpClientError> {
        let value = self
            .request_typed(
                RESOURCES_TEMPLATES_LIST,
                ListResourceTemplatesParams { cursor },
            )
            .await?;
        let result: ListResourceTemplatesResult =
            serde_json::from_value(value).map_err(McpClientError::Deserialize)?;
        Ok((
            result
                .resource_templates
                .into_iter()
                .map(ResourceTemplateDefinition::from)
                .collect(),
            result.next_cursor,
        ))
    }

    /// Lista todos os Prompts do servidor, atravessando as páginas de
    /// `prompts/list`.
    pub async fn list_prompts(&self) -> Result<Vec<PromptDefinition>, McpClientError> {
        let session = self.session_snapshot().await?;
        if !session.supports_prompts() {
            return Err(McpClientError::Unsupported(PROMPTS_LIST.to_owned()));
        }
        self.paginate(|cursor| async move { self.list_prompts_page(cursor).await })
            .await
    }

    /// Prompts do servidor com cache local.
    ///
    /// Comportamento análogo a [`tools`](Self::tools): a cache é
    /// preenchida na primeira chamada e invalidada pela notification
    /// `notifications/prompts/list_changed`.
    pub async fn prompts(&self) -> Result<Vec<PromptDefinition>, McpClientError> {
        if let Some(cached) = self.discovery.prompts() {
            return Ok(cached);
        }
        let prompts = self.list_prompts().await?;
        *self.discovery.prompts.lock().unwrap() = Some(prompts.clone());
        Ok(prompts)
    }

    /// Uma página de `prompts/list`.
    pub async fn list_prompts_page(
        &self,
        cursor: Option<String>,
    ) -> Result<(Vec<PromptDefinition>, Option<String>), McpClientError> {
        let value = self
            .request_typed(PROMPTS_LIST, ListPromptsParams { cursor })
            .await?;
        let result: ListPromptsResult =
            serde_json::from_value(value).map_err(McpClientError::Deserialize)?;
        Ok((
            result
                .prompts
                .into_iter()
                .map(PromptDefinition::from)
                .collect(),
            result.next_cursor,
        ))
    }

    /// Obtém um Prompt pelo nome, com os argumentos fornecidos.
    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult, McpClientError> {
        let session = self.session_snapshot().await?;
        if !session.supports_prompts() {
            return Err(McpClientError::Unsupported(PROMPTS_GET.to_owned()));
        }
        let payload = self
            .request_typed(
                PROMPTS_GET,
                GetPromptRequestParams::new(name.to_owned(), Some(arguments)),
            )
            .await?;
        serde_json::from_value(payload).map_err(McpClientError::Deserialize)
    }

    /// Capabilities anunciadas pelo servidor na negociação.
    ///
    /// Consulta local (sem nova chamada de rede); `None` antes do
    /// `initialize`.
    pub async fn server_capabilities(&self) -> Option<crate::protocol::ServerCapabilities> {
        let session = self.session.read().await;
        Some(session.as_ref()?.server_capabilities.clone())
    }

    /// Versão do protocolo MCP acordada na negociação.
    ///
    /// Consulta local (sem nova chamada de rede); `None` antes do
    /// `initialize`.
    pub async fn protocol_version(&self) -> Option<String> {
        let session = self.session.read().await;
        Some(session.as_ref()?.protocol_version.clone())
    }

    /// Identificação do servidor estabelecida na negociação.
    ///
    /// Consulta local (sem nova chamada de rede); `None` antes do
    /// `initialize`.
    pub async fn server_info(&self) -> Option<super::session::ServerInfo> {
        let session = self.session.read().await;
        let session = session.as_ref()?;
        Some(super::session::ServerInfo::from(&session.server_info))
    }

    /// Assina o stream de notifications recebidas do servidor.
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<JsonRpcNotification> {
        self.notifications_tx.subscribe()
    }

    /// Fecha o cliente, acordando requests pendentes e abortando o loop.
    ///
    /// Operações subsequentes retornam [`McpClientError::Closed`].
    pub async fn close(&self) -> Result<(), McpClientError> {
        let state = self.state.read().await;
        if *state == ClientState::Closed {
            return Ok(());
        }
        drop(state);

        *self.state.write().await = ClientState::Closed;
        self.request_manager.shutdown(None);

        if let Some(handle) = self.handle.lock().await.take() {
            handle.abort();
        }
        Ok(())
    }

    fn next_id(&self) -> JsonRpcId {
        JsonRpcId::Number(self.request_manager.next_id())
    }

    async fn require_initialized(&self) -> Result<(), McpClientError> {
        match *self.state.read().await {
            ClientState::Initialized => Ok(()),
            ClientState::Closed => Err(McpClientError::Closed),
            _ => Err(McpClientError::NotInitialized),
        }
    }

    /// Cópia da sessão negociada, exigindo `initialize` concluído.
    async fn session_snapshot(&self) -> Result<McpSession, McpClientError> {
        self.require_initialized().await?;
        self.session
            .read()
            .await
            .clone()
            .ok_or(McpClientError::NotInitialized)
    }

    async fn reset_initialization(&self) {
        let mut state = self.state.write().await;
        if *state == ClientState::Initializing {
            *state = ClientState::Connected;
        }
    }

    /// Envia um request de sessão e devolve o `result` bruto.
    ///
    /// Exige sessão iniciada (assim como [`Self::request`]), serializa os
    /// parâmetros e valida a resposta pelo método.
    async fn request_typed<P: Serialize>(
        &self,
        method: &str,
        params: P,
    ) -> Result<Value, McpClientError> {
        self.require_initialized().await?;
        let request =
            JsonRpcRequest::new(self.next_id(), method, Some(serde_json::to_value(params)?));
        let response = self
            .request_manager
            .send_request(request, self.config.request_timeout)
            .await?;
        payload_from_response(method, response)
    }

    /// Percorre todas as páginas de uma listagem paginada.
    ///
    /// `fetch(cursor)` traz uma página. A navegação termina quando o
    /// servidor devolve `None`, repete um cursor já visto (servidor
    /// malformado) ou estoura o limite de páginas — evitando loops
    /// infinitos com paginação degenerada.
    async fn paginate<Fut, Item>(
        &self,
        mut fetch: impl FnMut(Option<String>) -> Fut,
    ) -> Result<Vec<Item>, McpClientError>
    where
        Fut: std::future::Future<Output = Result<(Vec<Item>, Option<String>), McpClientError>>,
    {
        let mut collected = Vec::new();
        let mut cursor: Option<String> = None;
        let mut seen = HashSet::new();

        for _ in 0..MAX_PAGINATION_PAGES {
            let (items, next) = fetch(cursor.clone()).await?;
            collected.extend(items);

            match next {
                Some(next_cursor) if seen.insert(next_cursor.clone()) => {
                    cursor = Some(next_cursor);
                }
                _ => break,
            }
        }

        Ok(collected)
    }
}

/// Limite de páginas percorridas em listagens paginadas.
///
/// Proteção extra contra servidores que nunca repetem cursors.
const MAX_PAGINATION_PAGES: usize = 1024;

/// Caches locais de descoberta (`tools`, `resources`, `prompts`).
///
/// Compartilhada entre o [`McpClient`] e o loop do transporte: o
/// cliente preenche as entradas na primeira chamada de
/// `tools`/`resources`/`prompts`, e o loop as limpa quando recebe a
/// notification `.../list_changed` correspondente. Os `Mutex` são de
/// `std` e são mantidos apenas por microssegundos (nunca através de
/// `.await`).
///
/// As listagens `list_*` **não** passam pela cache: elas sempre
/// consultam o servidor. A cache existe apenas para o agente reutilizar
/// definições sem custo de rede.
pub(crate) struct DiscoveryCache {
    tools: SyncMutex<Option<Vec<ToolDefinition>>>,
    resources: SyncMutex<Option<Vec<ResourceDefinition>>>,
    prompts: SyncMutex<Option<Vec<PromptDefinition>>>,
}

impl DiscoveryCache {
    fn new() -> Self {
        Self {
            tools: SyncMutex::new(None),
            resources: SyncMutex::new(None),
            prompts: SyncMutex::new(None),
        }
    }

    fn tools(&self) -> Option<Vec<ToolDefinition>> {
        self.tools.lock().unwrap().clone()
    }

    fn resources(&self) -> Option<Vec<ResourceDefinition>> {
        self.resources.lock().unwrap().clone()
    }

    fn prompts(&self) -> Option<Vec<PromptDefinition>> {
        self.prompts.lock().unwrap().clone()
    }

    /// Invalida as entradas correspondentes à notification recebida.
    fn invalidate(&self, method: &str) {
        match method {
            NOTIFICATIONS_TOOLS_LIST_CHANGED => *self.tools.lock().unwrap() = None,
            NOTIFICATIONS_RESOURCES_LIST_CHANGED => *self.resources.lock().unwrap() = None,
            NOTIFICATIONS_PROMPTS_LIST_CHANGED => *self.prompts.lock().unwrap() = None,
            _ => {}
        }
    }
}

/// Executor de `tools/call` anexado às [`RemoteTool`]s.
///
/// Captura apenas o [`RequestManager`] e a configuração, mantendo-se
/// independente do tipo concreto de transporte (`McpClient<T>`) e,
/// portanto, sempre `Send + Sync`.
struct ClientToolCaller {
    request_manager: Arc<RequestManager>,
    config: Arc<ClientConfig>,
}

#[async_trait::async_trait]
impl ToolCaller for ClientToolCaller {
    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, McpClientError> {
        let params = CallToolRequestParams::new(name, Some(arguments));
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(self.request_manager.next_id()),
            TOOLS_CALL,
            Some(serde_json::to_value(params)?),
        );
        let response = self
            .request_manager
            .send_request(request, self.config.request_timeout)
            .await?;
        let payload = payload_from_response(TOOLS_CALL, response)?;
        serde_json::from_value(payload).map_err(McpClientError::Deserialize)
    }
}

/// Extrai o `result` bruto de uma `JsonRpcResponse`, ou o mapeia para
/// um erro tipado caso o servidor tenha respondido `error`.
fn payload_from_response(method: &str, response: JsonRpcResponse) -> Result<Value, McpClientError> {
    let id = response.id.clone();
    if let Some(error) = response.error {
        return Err(McpClientError::Server {
            method: method.to_owned(),
            id,
            code: error.code,
            message: error.message,
            data: error.data,
        });
    }
    response
        .result
        .ok_or_else(|| McpClientError::InvalidResponse {
            method: method.to_owned(),
            id,
            reason: "resposta sem `result` nem `error`".to_owned(),
        })
}

impl<T> fmt::Debug for McpClient<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct View {
            state: Option<ClientState>,
            session: Option<McpSession>,
        }
        let view = View {
            state: self.state.try_read().ok().map(|s| *s),
            session: self.session.try_read().ok().and_then(|g| g.clone()),
        };
        f.debug_struct("McpClient").field("view", &view).finish()
    }
}

/// Builder do [`McpClient`].
///
/// ```
/// use gerax_mcp::client::McpClientBuilder;
///
/// let builder = McpClientBuilder::new()
///     .name("gerax-agent")
///     .version("0.1.0");
/// assert!(builder.build().is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct McpClientBuilder {
    name: String,
    version: String,
    capabilities: ClientCapabilities,
    request_timeout: Duration,
}

impl Default for McpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClientBuilder {
    /// Cria um builder com defaults seguros.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            capabilities: ClientCapabilities::default(),
            request_timeout: Duration::from_secs(30),
        }
    }

    /// Define o nome da implementação do cliente.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Define a versão da implementação do cliente.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Define as capabilities declaradas pelo cliente.
    ///
    /// Por padrão, nenhuma capability é anunciada.
    pub fn capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Define o timeout de cada request.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Valida a configuração.
    ///
    /// Exige `name` e `version` não vazios.
    pub fn build(self) -> Result<Self, McpClientError> {
        if self.name.trim().is_empty() {
            return Err(McpClientError::invalid_config("`name` é obrigatório"));
        }
        if self.version.trim().is_empty() {
            return Err(McpClientError::invalid_config("`version` é obrigatória"));
        }
        Ok(self)
    }

    /// Conecta a um servidor MCP através do transporte fornecido.
    ///
    /// Inicia a tarefa interna de leitura/escrita e devolve um cliente
    /// pronto para o handshake `initialize`.
    pub async fn connect<T: Transport + 'static>(
        self,
        transport: T,
    ) -> Result<McpClient<T>, McpClientError> {
        let config = Arc::new(ClientConfig {
            name: self.name,
            version: self.version,
            capabilities: self.capabilities,
            request_timeout: self.request_timeout,
        });

        let (outbound_tx, outbound_rx) = mpsc::channel(64);
        let (notifications_tx, _) = broadcast::channel(64);
        let request_manager = RequestManager::new(outbound_tx);
        let discovery = Arc::new(DiscoveryCache::new());

        let handle = tokio::spawn(run_transport_loop(
            transport,
            outbound_rx,
            request_manager.clone(),
            notifications_tx.clone(),
            discovery.clone(),
        ));

        Ok(McpClient {
            request_manager,
            state: RwLock::new(ClientState::Connected),
            session: RwLock::new(None),
            config,
            notifications_tx,
            discovery,
            handle: Mutex::new(Some(handle)),
            _transport: PhantomData,
        })
    }
}

/// Mensagens que o loop de escrita precisa entregar ao transporte.
#[derive(Debug)]
pub(crate) enum OutboundMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

/// Loop interno que dirige o transporte e distribui mensagens.
///
/// Escrita: enquadra requests/notifications no `Transport`. Leitura:
/// delega a distribuição ao [`ResponseDispatcher`]. Notifications de
/// mudança (`.../list_changed`) invalidam as caches de descoberta. Ao
/// fim da stream (EOF) ou em erro, encerra o manager, acordando
/// requests pendentes.
pub(crate) async fn run_transport_loop<T: Transport>(
    mut transport: T,
    mut outbound_rx: mpsc::Receiver<OutboundMessage>,
    request_manager: Arc<RequestManager>,
    notifications_tx: broadcast::Sender<JsonRpcNotification>,
    discovery: Arc<DiscoveryCache>,
) {
    let mut transport_error: Option<TransportError> = None;
    let dispatcher = ResponseDispatcher::new(request_manager.clone(), notifications_tx);

    loop {
        tokio::select! {
            message = outbound_rx.recv() => {
                match message {
                    Some(OutboundMessage::Request(request)) => {
                        if let Err(error) = transport.send(JsonRpcMessage::Request(request)).await {
                            transport_error = Some(error);
                            break;
                        }
                    }
                    Some(OutboundMessage::Notification(notification)) => {
                        if let Err(error) = transport.notify(notification).await {
                            transport_error = Some(error);
                            break;
                        }
                    }
                    None => break,
                }
            }
            received = transport.receive() => {
                match received {
                    Ok(None) => break,
                    Ok(Some(message)) => {
                        if let JsonRpcMessage::Notification(notification) = &message {
                            discovery.invalidate(&notification.method);
                        }
                        dispatcher.dispatch(message)
                    }
                    Err(error) => {
                        transport_error = Some(error);
                        break;
                    }
                }
            }
        }
    }

    request_manager.shutdown(transport_error);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::mock::{MockServerHandle, MockTransport};
    use crate::protocol::{ImplementationInfo, InitializeResult, ServerCapabilities};
    use serde_json::json;

    fn builder() -> McpClientBuilder {
        McpClient::<MockTransport>::builder()
            .name("gerax-agent")
            .version("0.1.0")
    }

    fn init_result(server_name: &str) -> Value {
        serde_json::to_value(InitializeResult::new(
            MCP_PROTOCOL_VERSION,
            ServerCapabilities::default()
                .with_tools()
                .with_resources()
                .with_prompts(),
            ImplementationInfo::new(server_name, "0.1.0"),
        ))
        .unwrap()
    }

    #[test]
    fn builder_requires_name_and_version() {
        assert!(
            McpClientBuilder::new()
                .name("x")
                .version("1")
                .build()
                .is_ok()
        );
        assert!(matches!(
            McpClientBuilder::new().version("1").build(),
            Err(McpClientError::InvalidConfig(_))
        ));
        assert!(matches!(
            McpClientBuilder::new().name("x").build(),
            Err(McpClientError::InvalidConfig(_))
        ));
    }

    #[test]
    fn builder_defaults_capabilities_and_timeout() {
        let builder = McpClientBuilder::new()
            .name("x")
            .version("1")
            .build()
            .unwrap();

        assert_eq!(builder.request_timeout, Duration::from_secs(30));
        assert_eq!(builder.capabilities, ClientCapabilities::default());
    }

    #[tokio::test]
    async fn initialize_performs_handshake_and_marks_initialized() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();

        let initialize = client.initialize();
        let handshake = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "initialize");
            let Some(params) = request.params else {
                panic!("initialize exige params");
            };
            assert_eq!(params["protocolVersion"], MCP_PROTOCOL_VERSION);
            assert_eq!(params["clientInfo"]["name"], "gerax-agent");
            server.respond(request.id, init_result("escola-cqrs"));
            let notification = server.next_notification().await;
            assert_eq!(notification, "notifications/initialized");
        };

        let (session, _) = tokio::join!(initialize, handshake);
        let session = session.unwrap();

        assert_eq!(session.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(session.server_info.name, "escola-cqrs");
        assert!(session.supports_tools());

        assert_eq!(client.session().await, Some(session.clone()));
    }

    #[tokio::test]
    async fn request_before_initialize_is_not_initialized() {
        let (transport, _server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();

        let error = client.request("tools/list", None).await.unwrap_err();

        assert!(matches!(error, McpClientError::NotInitialized));
    }

    #[tokio::test]
    async fn initialize_twice_is_already_initialized() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();

        let initialize = client.initialize();
        let handshake = async move {
            let request = server.next_request().await;
            server.respond(request.id, init_result("srv"));
            server.next_notification().await;
        };
        let (result, _) = tokio::join!(initialize, handshake);
        result.unwrap();

        let error = client.initialize().await.unwrap_err();
        assert!(matches!(error, McpClientError::AlreadyInitialized));
    }

    #[tokio::test]
    async fn timeout_is_reported_when_server_never_responds() {
        let (transport, _server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();

        let error = client
            .request_manager
            .send_request(
                JsonRpcRequest::new(JsonRpcId::Number(1), "ping", None),
                Duration::from_millis(30),
            )
            .await
            .unwrap_err();

        assert!(matches!(error, McpClientError::Timeout));
    }

    #[tokio::test]
    async fn server_error_is_mapped_with_diagnostics() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();

        let initialize = client.initialize();
        let handshake = async move {
            let request = server.next_request().await;
            server.respond_error(request.id, -32602, "protocol version too new");
        };
        let (session, _) = tokio::join!(initialize, handshake);

        let error = session.unwrap_err();
        match error {
            McpClientError::Server {
                method,
                code,
                message,
                ..
            } => {
                assert_eq!(method, "initialize");
                assert_eq!(code, -32602);
                assert_eq!(message, "protocol version too new");
            }
            other => panic!("esperava Server, recebi {other:?}"),
        }

        // Após falha, o cliente volta a permitir nova tentativa.
        let state = *client.state.read().await;
        assert_eq!(state, ClientState::Connected);
    }

    #[tokio::test]
    async fn concurrent_requests_are_correlated_by_id() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let a = client.request("echo_a", None);
        let b = client.request("echo_b", None);
        let c = client.request("echo_c", None);

        let read_and_respond = async move {
            let mut ids = Vec::new();
            for _ in 0..3 {
                let request = server.next_request().await;
                ids.push(request.id.clone());
            }
            // Responde na ordem inversa do envio.
            for id in ids.iter().rev() {
                server.respond(id.clone(), json!({ "ok": id.clone() }));
            }
        };

        let (a, b, c, _) = tokio::join!(a, b, c, read_and_respond);

        assert!(a.unwrap().is_object());
        assert!(b.unwrap().is_object());
        assert!(c.unwrap().is_object());
        assert_eq!(client.request_manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn concurrent_tool_calls_complete_when_server_responds_out_of_order() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let a = client.call_tool("tool_a", json!({}));
        let b = client.call_tool("tool_b", json!({}));
        let c = client.call_tool("tool_c", json!({}));

        let read_and_respond = async {
            let mut ids = Vec::new();
            for _ in 0..3 {
                let request = server.next_request().await;
                assert_eq!(request.method, "tools/call");
                ids.push(request.id.clone());
            }
            // O servidor responde propositalmente fora de ordem.
            for (index, id) in ids.iter().rev().enumerate() {
                server.respond(
                    id.clone(),
                    json!({
                        "content": [{ "type": "text", "text": format!("result_{index}") }]
                    }),
                );
            }
        };

        let (a, b, c, _) = tokio::join!(a, b, c, read_and_respond);

        assert_eq!(a.unwrap().content.len(), 1);
        assert_eq!(b.unwrap().content.len(), 1);
        assert_eq!(c.unwrap().content.len(), 1);
        assert_eq!(client.request_manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn list_changed_notification_makes_next_list_tools_refetch() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        // Primeira listagem: servidor conhece uma Tool.
        let list = client.list_tools();
        let respond = async {
            let request = server.next_request().await;
            server.respond(
                request.id,
                json!({ "tools": [{ "name": "echo", "inputSchema": { "type": "object" } }] }),
            );
        };
        let (tools, _) = tokio::join!(list, respond);
        assert_eq!(tools.unwrap().len(), 1);

        // `tools/list_changed` invalida a cache de descoberta.
        server.push(JsonRpcMessage::Notification(
            crate::protocol::tools_list_changed_notification(),
        ));
        for _ in 0..100 {
            if client.discovery.tools().is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            client.discovery.tools().is_none(),
            "tools/list_changed deveria invalidar a cache"
        );

        // A listagem seguinte refaz a busca e enxerga a Tool nova.
        let list = client.list_tools();
        let respond = async {
            let request = server.next_request().await;
            assert_eq!(request.method, "tools/list");
            server.respond(
                request.id,
                json!({
                    "tools": [
                        { "name": "echo", "inputSchema": { "type": "object" } },
                        { "name": "nova", "inputSchema": { "type": "object" } }
                    ]
                }),
            );
        };
        let (tools, _) = tokio::join!(list, respond);
        assert_eq!(tools.unwrap().len(), 2);
    }

    #[test]
    fn client_is_send_and_sync_for_any_transport() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<McpClient<MockTransport>>();
    }

    #[tokio::test]
    async fn client_is_shareable_across_tasks_via_arc() {
        let (transport, mut server) = MockTransport::duplex();
        let client = Arc::new(builder().connect(transport).await.unwrap());
        init_client(&client, &mut server).await;

        let spawned = {
            let client = client.clone();
            tokio::spawn(async move {
                let a = client.call_tool("tool_a", json!({ "n": 1 }));
                let b = client.call_tool("tool_b", json!({ "n": 2 }));
                let (a, b) = tokio::join!(a, b);
                (a.unwrap(), b.unwrap())
            })
        };

        let mut ids = Vec::new();
        for _ in 0..2 {
            let request = server.next_request().await;
            assert_eq!(request.method, "tools/call");
            ids.push(request.id.clone());
        }
        for id in ids.iter().rev() {
            server.respond(
                id.clone(),
                json!({ "content": [{ "type": "text", "text": "ok" }] }),
            );
        }

        let (a, b) = spawned.await.unwrap();
        assert_eq!(a.content.len(), 1);
        assert_eq!(b.content.len(), 1);
        assert_eq!(client.request_manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn call_tool_propagates_server_error_with_diagnostics() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let call = client.call_tool("soma", json!({ "a": 1 }));
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "tools/call");
            server.respond_error(request.id, -32602, "argumentos inválidos");
        };
        let (result, _) = tokio::join!(call, respond);

        match result.unwrap_err() {
            McpClientError::Server {
                method,
                id,
                code,
                message,
                data,
            } => {
                assert_eq!(method, "tools/call");
                assert!(matches!(id, JsonRpcId::Number(_)));
                assert_eq!(code, -32602);
                assert_eq!(message, "argumentos inválidos");
                assert!(data.is_none());
            }
            other => panic!("esperava Server, recebi {other:?}"),
        }
    }

    #[tokio::test]
    async fn invalid_response_without_result_or_error_is_detected() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let call = client.request("weird", None);
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "weird");
            let malformed = serde_json::from_value::<JsonRpcMessage>(json!({
                "jsonrpc": "2.0",
                "id": request.id,
            }))
            .unwrap();
            server.push(malformed);
        };
        let (result, _) = tokio::join!(call, respond);

        match result.unwrap_err() {
            McpClientError::InvalidResponse { method, .. } => {
                assert_eq!(method, "weird");
            }
            other => panic!("esperava InvalidResponse, recebi {other:?}"),
        }
    }

    #[tokio::test]
    async fn tools_cache_is_reused_and_invalidated_by_notification() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let tools = client.tools();
        let respond = async {
            let request = server.next_request().await;
            assert_eq!(request.method, "tools/list");
            server.respond(
                request.id,
                json!({ "tools": [{ "name": "echo", "inputSchema": { "type": "object" } }] }),
            );
        };
        let (tools, _) = tokio::join!(tools, respond);
        assert_eq!(tools.unwrap().len(), 1);

        // Cache quente: `tools()` não deve emitir novo request.
        assert_eq!(client.tools().await.unwrap().len(), 1);
        let silence = tokio::time::timeout(Duration::from_millis(50), server.next_request()).await;
        assert!(
            silence.is_err(),
            "cache quente deveria evitar request ao servidor"
        );

        // Notification invalida a cache.
        server.push(JsonRpcMessage::Notification(
            crate::protocol::tools_list_changed_notification(),
        ));
        for _ in 0..100 {
            if client.discovery.tools().is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            client.discovery.tools().is_none(),
            "tools/list_changed deveria invalidar a cache"
        );

        // Próxima chamada busca de novo no servidor.
        let tools = client.tools();
        let respond = async {
            let request = server.next_request().await;
            server.respond(
                request.id,
                json!({ "tools": [{ "name": "echo", "inputSchema": { "type": "object" } }] }),
            );
        };
        let (tools, _) = tokio::join!(tools, respond);
        assert_eq!(tools.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn resources_and_prompts_caches_are_invalidated_by_notifications() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let resources = client.resources();
        let respond = async {
            let request = server.next_request().await;
            server.respond(
                request.id,
                json!({ "resources": [{ "uri": "file:///a.txt", "name": "a" }] }),
            );
        };
        let (resources, _) = tokio::join!(resources, respond);
        assert_eq!(resources.unwrap().len(), 1);
        assert!(client.discovery.resources().is_some());

        server.push(JsonRpcMessage::Notification(
            crate::protocol::resources_list_changed_notification(),
        ));
        for _ in 0..100 {
            if client.discovery.resources().is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            client.discovery.resources().is_none(),
            "resources/list_changed deveria invalidar a cache"
        );

        let prompts = client.prompts();
        let respond = async {
            let request = server.next_request().await;
            server.respond(request.id, json!({ "prompts": [{ "name": "p1" }] }));
        };
        let (prompts, _) = tokio::join!(prompts, respond);
        assert_eq!(prompts.unwrap().len(), 1);
        assert!(client.discovery.prompts().is_some());

        server.push(JsonRpcMessage::Notification(
            crate::protocol::prompts_list_changed_notification(),
        ));
        for _ in 0..100 {
            if client.discovery.prompts().is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            client.discovery.prompts().is_none(),
            "prompts/list_changed deveria invalidar a cache"
        );
    }

    async fn run_fake_mcp_server(mut server: MockServerHandle) {
        loop {
            let message = server.next_message().await;
            let JsonRpcMessage::Request(request) = message else {
                continue;
            };
            let result = match request.method.as_str() {
                "initialize" => init_result("fake"),
                "tools/list" => json!({
                    "tools": [{ "name": "echo", "inputSchema": { "type": "object" } }]
                }),
                "tools/call" => json!({
                    "content": [{ "type": "text", "text": "eco" }]
                }),
                "resources/list" => json!({
                    "resources": [{ "uri": "file:///a.txt", "name": "a" }]
                }),
                "resources/read" => json!({
                    "contents": [{ "uri": "file:///a.txt", "type": "text", "text": "ola" }]
                }),
                "resources/templates/list" => json!({ "resourceTemplates": [] }),
                "prompts/list" => json!({ "prompts": [{ "name": "greeting" }] }),
                "prompts/get" => json!({
                    "messages": [{ "role": "user", "content": { "type": "text", "text": "oi" } }]
                }),
                other => {
                    server.respond_error(request.id, -32601, other);
                    continue;
                }
            };
            server.respond(request.id, result);
        }
    }

    #[tokio::test]
    async fn full_workflow_over_fake_server() {
        let (transport, server) = MockTransport::duplex();
        let server_task = tokio::spawn(async move { run_fake_mcp_server(server).await });

        let session = {
            let client = builder().connect(transport).await.unwrap();
            let session = client.initialize().await.unwrap();
            assert_eq!(session.server_info.name, "fake");

            let tools = client.list_tools().await.unwrap();
            assert_eq!(tools.len(), 1);

            let result = client.call_tool("echo", json!({})).await.unwrap();
            assert_eq!(result.content.len(), 1);

            let resources = client.list_resources().await.unwrap();
            assert_eq!(resources.len(), 1);
            assert_eq!(resources[0].name, "a");

            let contents = client.read_resource("file:///a.txt").await.unwrap();
            assert_eq!(contents.contents.len(), 1);

            let templates = client.list_resource_templates().await.unwrap();
            assert!(templates.is_empty());

            let prompts = client.list_prompts().await.unwrap();
            assert_eq!(prompts.len(), 1);

            let prompt = client.get_prompt("greeting", json!({})).await.unwrap();
            assert_eq!(prompt.messages.len(), 1);

            client.close().await.unwrap();
            session
        };
        assert_eq!(session.server_info.name, "fake");

        server_task.abort();
    }

    #[tokio::test]
    async fn transport_close_wakes_pending_requests() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let request = client.request("tools/list", None);
        let ensure = async move {
            let _ = server.next_request().await;
            drop(server);
        };
        let (result, _) = tokio::join!(request, ensure);

        assert!(matches!(result, Err(McpClientError::Closed)));
    }

    #[tokio::test]
    async fn close_rejects_new_operations() {
        let (transport, _server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();

        client.close().await.unwrap();
        client.close().await.unwrap();

        let error = client.request("tools/list", None).await.unwrap_err();
        assert!(matches!(error, McpClientError::Closed));

        // Operações de sessão também são rejeitadas após o fechamento,
        // sem tentar usar o transporte.
        let error = client.call_tool("echo", json!({})).await.unwrap_err();
        assert!(matches!(error, McpClientError::Closed));

        let error = client.list_prompts().await.unwrap_err();
        assert!(matches!(error, McpClientError::Closed));

        let error = client.read_resource("file:///x").await.unwrap_err();
        assert!(matches!(error, McpClientError::Closed));
    }

    #[tokio::test]
    async fn notifications_received_from_server_are_forwarded() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let mut rx = client.subscribe_notifications();
        server.push(JsonRpcMessage::Notification(
            crate::protocol::tools_list_changed_notification(),
        ));

        let received = rx.recv().await.unwrap();
        assert_eq!(received.method, "notifications/tools/list_changed");
    }

    #[tokio::test]
    async fn list_tools_returns_definitions() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let list = client.list_tools();
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "tools/list");
            server.respond(
                request.id,
                json!({
                    "tools": [
                        { "name": "echo", "description": "Ecoa", "inputSchema": { "type": "object" } },
                        { "name": "soma", "inputSchema": { "type": "object" } }
                    ]
                }),
            );
        };
        let (tools, _) = tokio::join!(list, respond);
        let tools = tools.unwrap();

        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "echo");
        assert_eq!(tools[0].description.as_deref(), Some("Ecoa"));
        assert_eq!(tools[0].input_schema, json!({ "type": "object" }));
        assert_eq!(tools[1].name, "soma");
        assert_eq!(tools[1].description, None);
    }

    #[tokio::test]
    async fn list_tools_fetches_all_pages() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let list = client.list_tools();
        let respond = async move {
            for page in 1..=3 {
                let request = server.next_request().await;
                assert_eq!(request.method, "tools/list");
                let result = if page < 3 {
                    json!({
                        "tools": [{ "name": format!("tool_{page}"), "inputSchema": { "type": "object" } }],
                        "nextCursor": format!("cursor_{page}")
                    })
                } else {
                    json!({
                        "tools": [{ "name": "tool_3", "inputSchema": { "type": "object" } }]
                    })
                };
                server.respond(request.id, result);
            }
        };
        let (tools, _) = tokio::join!(list, respond);
        let tools = tools.unwrap();

        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].name, "tool_1");
        assert_eq!(tools[2].name, "tool_3");
    }

    #[tokio::test]
    async fn list_tools_stops_on_repeated_cursor() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let list = client.list_tools();
        let respond = async move {
            // Servidor malformado: repete o mesmo cursor indefinidamente.
            for _ in 0..2 {
                let request = server.next_request().await;
                server.respond(
                    request.id,
                    json!({
                        "tools": [{ "name": "x", "inputSchema": { "type": "object" } }],
                        "nextCursor": "same"
                    }),
                );
            }
        };
        let (tools, _) = tokio::join!(list, respond);
        let tools = tools.unwrap();

        assert_eq!(tools.len(), 2, "cursor repetido encerra a navegação");
    }

    #[tokio::test]
    async fn call_tool_returns_result() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let call = client.call_tool("create_aluno", json!({ "nome": "João" }));
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "tools/call");
            assert_eq!(request.params.as_ref().unwrap()["name"], "create_aluno");
            assert_eq!(
                request.params.as_ref().unwrap()["arguments"]["nome"],
                "João"
            );
            server.respond(
                request.id,
                json!({
                    "content": [{ "type": "text", "text": "aluno criado" }]
                }),
            );
        };
        let (result, _) = tokio::join!(call, respond);
        let result = result.unwrap();

        assert_eq!(result.content.len(), 1);
        assert!(!result.is_error);
    }

    #[tokio::test]
    async fn remote_tools_are_invocable() {
        let (transport, mut server) = MockTransport::duplex();
        let server_task = tokio::spawn(async move {
            let request = server.next_request().await;
            server.respond(request.id, init_result("srv"));
            server.next_notification().await;

            let request = server.next_request().await;
            server.respond(
                request.id,
                json!({
                    "tools": [{ "name": "echo", "description": "Ecoa", "inputSchema": { "type": "object" } }]
                }),
            );
            let request = server.next_request().await;
            assert_eq!(request.method, "tools/call");
            assert_eq!(request.params.as_ref().unwrap()["name"], "echo");
            server.respond(
                request.id,
                json!({ "content": [{ "type": "text", "text": "ecoa" }] }),
            );
        });

        let client = builder().connect(transport).await.unwrap();
        let session = client.initialize().await.unwrap();
        assert_eq!(session.server_info.name, "srv");

        let tools = client.remote_tools().await.unwrap();
        assert_eq!(tools.len(), 1);

        let result = tools[0].call(json!({ "msg": "oi" })).await.unwrap();
        assert_eq!(result.content.len(), 1);

        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn list_resources_returns_definitions() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let list = client.list_resources();
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "resources/list");
            server.respond(
                request.id,
                json!({
                    "resources": [
                        { "uri": "file:///ola.txt", "name": "ola.txt", "mimeType": "text/plain" }
                    ]
                }),
            );
        };
        let (resources, _) = tokio::join!(list, respond);
        let resources = resources.unwrap();

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "file:///ola.txt");
        assert_eq!(resources[0].mime_type.as_deref(), Some("text/plain"));
    }

    #[tokio::test]
    async fn read_resource_returns_contents() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let read = client.read_resource("file:///ola.txt");
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "resources/read");
            assert_eq!(request.params.as_ref().unwrap()["uri"], "file:///ola.txt");
            server.respond(
                request.id,
                json!({
                    "contents": [{ "uri": "file:///ola.txt", "text": "olá" }]
                }),
            );
        };
        let (result, _) = tokio::join!(read, respond);
        let result = result.unwrap();

        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.contents[0].uri(), "file:///ola.txt");
    }

    #[tokio::test]
    async fn list_resource_templates_returns_definitions() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let list = client.list_resource_templates();
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "resources/templates/list");
            server.respond(
                request.id,
                json!({
                    "resourceTemplates": [
                        { "uriTemplate": "file:///{path}", "name": "Arquivo", "mimeType": "text/plain" }
                    ],
                    "nextCursor": "c2"
                }),
            );
            let request = server.next_request().await;
            server.respond(
                request.id,
                json!({
                    "resourceTemplates": [
                        { "uriTemplate": "db:///{schema}/{table}", "name": "Tabela" }
                    ]
                }),
            );
        };
        let (templates, _) = tokio::join!(list, respond);
        let templates = templates.unwrap();

        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].uri_template, "file:///{path}");
        assert_eq!(templates[0].mime_type.as_deref(), Some("text/plain"));
        assert_eq!(templates[1].uri_template, "db:///{schema}/{table}");
    }

    #[tokio::test]
    async fn list_prompts_returns_definitions() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let list = client.list_prompts();
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "prompts/list");
            server.respond(
                request.id,
                json!({
                    "prompts": [{ "name": "greeting", "description": "Saudação" }]
                }),
            );
        };
        let (prompts, _) = tokio::join!(list, respond);
        let prompts = prompts.unwrap();

        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "greeting");
        assert_eq!(prompts[0].description.as_deref(), Some("Saudação"));
    }

    #[tokio::test]
    async fn list_prompts_fetches_all_pages() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let list = client.list_prompts();
        let respond = async move {
            let request = server.next_request().await;
            server.respond(
                request.id,
                json!({
                    "prompts": [{ "name": "p1", "description": "Um" }],
                    "nextCursor": "c1"
                }),
            );
            let request = server.next_request().await;
            server.respond(request.id, json!({ "prompts": [{ "name": "p2" }] }));
        };
        let (prompts, _) = tokio::join!(list, respond);
        let prompts = prompts.unwrap();

        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0].name, "p1");
        assert_eq!(prompts[1].name, "p2");
    }

    #[tokio::test]
    async fn get_prompt_returns_messages() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let call = client.get_prompt("greeting", json!({ "name": "Ana" }));
        let respond = async move {
            let request = server.next_request().await;
            assert_eq!(request.method, "prompts/get");
            assert_eq!(request.params.as_ref().unwrap()["name"], "greeting");
            assert_eq!(request.params.as_ref().unwrap()["arguments"]["name"], "Ana");
            server.respond(
                request.id,
                json!({
                    "description": "Saudação",
                    "messages": [
                        { "role": "user", "content": { "type": "text", "text": "Olá, Ana" } }
                    ]
                }),
            );
        };
        let (result, _) = tokio::join!(call, respond);
        let result = result.unwrap();

        assert_eq!(result.messages.len(), 1);
    }

    #[tokio::test]
    async fn server_info_and_capabilities_are_read_locally() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();
        init_client(&client, &mut server).await;

        let info = client.server_info().await.unwrap();
        assert_eq!(info.name, "srv");
        assert_eq!(info.version, "0.1.0");

        let capabilities = client.server_capabilities().await.unwrap();
        assert!(capabilities.tools.is_some());
        assert!(capabilities.resources.is_some());
        assert!(capabilities.prompts.is_some());

        assert_eq!(
            client.protocol_version().await.as_deref(),
            Some(MCP_PROTOCOL_VERSION)
        );
    }

    #[tokio::test]
    async fn unsupported_capability_is_rejected() {
        let (transport, mut server) = MockTransport::duplex();
        let client = builder().connect(transport).await.unwrap();

        let initialize = client.initialize();
        let handshake = async move {
            let request = server.next_request().await;
            // Servidor sem capability `tools`.
            server.respond(
                request.id,
                serde_json::to_value(InitializeResult::new(
                    MCP_PROTOCOL_VERSION,
                    ServerCapabilities::default(),
                    ImplementationInfo::new("srv", "0.1.0"),
                ))
                .unwrap(),
            );
            server.next_notification().await;
        };
        let (session, _) = tokio::join!(initialize, handshake);
        session.unwrap();

        let error = client.call_tool("echo", json!({})).await.unwrap_err();
        assert!(
            matches!(error, McpClientError::Unsupported(ref op) if op == TOOLS_CALL),
            "esperava Unsupported, recebi {error:?}"
        );

        // Nenhum request foi emitido ao servidor.
        let error = client.list_tools().await.unwrap_err();
        assert!(matches!(error, McpClientError::Unsupported(_)));
    }

    #[tokio::test]
    async fn process_exit_wakes_pending_requests() {
        let transport = crate::transport::StdioTransport::command("sh")
            .args(["-c", "exit 1"])
            .build()
            .await
            .unwrap();
        let client = builder().connect(transport).await.unwrap();

        let error = client.initialize().await.unwrap_err();

        assert!(
            matches!(error, McpClientError::Transport(_)),
            "esperava Transport(ProcessExited), recebi {error:?}"
        );
    }

    async fn init_client(client: &McpClient<MockTransport>, server: &mut MockServerHandle) {
        let initialize = client.initialize();
        let handshake = async {
            let request = server.next_request().await;
            server.respond(request.id, init_result("srv"));
            server.next_notification().await;
        };
        let (session, _) = tokio::join!(initialize, handshake);
        session.unwrap();
    }
}
