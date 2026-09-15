//! [`McpHttpServer`]: ponte entre HTTP e o [`McpServer`] do `gerax-mcp`.
//!
//! Implementa o transporte **Streamable HTTP** com uma única sessão
//! ativa por instância: o lifecycle do [`McpServer`] é global e um novo
//! ciclo de `initialize` só começa depois que a sessão é encerrada via
//! HTTP DELETE.

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, ORIGIN};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use bytes::Bytes;
use futures::{StreamExt, stream};
use gerax_mcp::JsonRpcMessage;
use gerax_mcp::protocol::initialize::INITIALIZE;
use gerax_mcp::{McpDispatcher, McpServer};
use serde_json::{Value, json};
use tokio::sync::broadcast::error::RecvError;

use crate::SessionStore;
use crate::config::{HttpTransportConfig, OriginPolicy};
use crate::session::SessionId;
use crate::sse::{SseEvent, event_sequence, prime_event};

const MCP_SESSION_HEADER: &str = "mcp-session-id";
const MCP_PROTOCOL_VERSION_HEADER: &str = "mcp-protocol-version";

/// Resultado de uma operação HTTP, materializado a partir da lógica MCP.
///
/// Esse tipo descreve decisões de transporte independentes de framework,
/// permitindo reutilização em adapters futuros (axum, actix, poem, salvo).
#[derive(Debug, Clone, PartialEq)]
pub enum HttpOutcome {
    /// Resposta JSON-RPC única, em `application/json`.
    Json {
        /// Status HTTP da resposta.
        status: StatusCode,
        /// Corpo JSON-RPC.
        body: Value,
        /// Identificador da sessão ativa, quando existir.
        session_id: Option<SessionId>,
    },
    /// 202 Accepted, corpo vazio: notification/response do cliente aceito.
    Accepted {
        /// Identificador da sessão ativa, quando existir.
        session_id: Option<SessionId>,
    },
    /// Erro de transporte (ex.: 400/404/403) com corpo JSON-RPC sem id.
    Error {
        /// Status HTTP do erro.
        status: StatusCode,
        /// Código JSON-RPC do erro.
        code: i32,
        /// Mensagem legível do erro.
        message: String,
    },
    /// Corpo `text/event-stream` já serializado (POST finito).
    Sse {
        /// Enquadramento SSE pronto para envio.
        body: String,
        /// Identificador da sessão ativa, quando existir.
        session_id: Option<SessionId>,
    },
    /// 405 Method Not Allowed.
    MethodNotAllowed,
    /// 204/404 genérico sem corpo.
    Empty {
        /// Status HTTP da resposta.
        status: StatusCode,
    },
}

/// Ponte HTTP MCP sobre um [`McpServer`].
#[derive(Clone)]
pub struct McpHttpServer {
    server: Arc<McpServer>,
    config: Arc<HttpTransportConfig>,
    sessions: Arc<SessionStore>,
    event_sequence: Arc<AtomicU64>,
}

impl McpHttpServer {
    /// Cria um builder a partir de um servidor MCP.
    pub fn for_server(server: impl Into<Arc<McpServer>>) -> McpHttpServerBuilder {
        McpHttpServerBuilder::default().server(server)
    }

    /// Servidor MCP subjacente.
    pub fn server(&self) -> &Arc<McpServer> {
        &self.server
    }

    /// Configuração do transporte HTTP.
    pub fn config(&self) -> &HttpTransportConfig {
        &self.config
    }

    /// Sessão ativa, quando existir.
    pub fn session_id(&self) -> Option<SessionId> {
        self.sessions.current()
    }

    /// Constrói o `Router` axum com o endpoint configurado.
    pub fn router(&self) -> Router {
        Router::new()
            .route(
                &self.config.endpoint,
                get(get_handler).post(post_handler).delete(delete_handler),
            )
            .with_state(self.clone())
    }

    /// Processa uma requisição HTTP `POST` (mensagem JSON-RPC).
    pub async fn handle_post(&self, headers: &HeaderMap, body: Bytes) -> HttpOutcome {
        if let Err(error) = self.validate_headers(headers) {
            return error;
        }

        let message = match parse_message(&body) {
            Ok(message) => message,
            Err(error) => return error,
        };

        match message {
            JsonRpcMessage::Request(request) => self.handle_request_post(headers, request).await,
            JsonRpcMessage::Notification(notification) => {
                let outcome = match self.require_session(headers) {
                    Ok(session) => HttpOutcome::Accepted {
                        session_id: session,
                    },
                    Err(error) => error,
                };
                if matches!(outcome, HttpOutcome::Accepted { .. }) {
                    let _ = McpDispatcher::new(self.server.clone())
                        .dispatch(JsonRpcMessage::Notification(notification))
                        .await;
                }
                outcome
            }
            JsonRpcMessage::Response(_) => match self.require_session(headers) {
                Ok(session) => HttpOutcome::Accepted {
                    session_id: session,
                },
                Err(error) => error,
            },
        }
    }

    /// Processa uma requisição HTTP `GET`: stream SSE de notifications.
    pub async fn handle_get(&self, headers: &HeaderMap) -> HttpOutcome {
        if let Err(error) = self.validate_headers(headers) {
            return error;
        }
        if !accepts_sse(headers) || !self.config.allow_client_stream {
            return HttpOutcome::MethodNotAllowed;
        }
        match self.require_session(headers) {
            Ok(_) => HttpOutcome::Sse {
                body: String::new(),
                session_id: self.sessions.current(),
            },
            Err(error) => error,
        }
    }

    /// Processa uma requisição HTTP `DELETE`: encerra a sessão ativa.
    pub async fn handle_delete(&self, headers: &HeaderMap) -> HttpOutcome {
        if let Err(error) = self.validate_headers(headers) {
            return error;
        }
        if !self.config.allow_session_delete {
            return HttpOutcome::MethodNotAllowed;
        }
        match self.require_session(headers) {
            Ok(_) => {
                self.sessions.terminate();
                self.server.end_session();
                HttpOutcome::Empty {
                    status: StatusCode::NO_CONTENT,
                }
            }
            Err(error) => error,
        }
    }

    async fn handle_request_post(
        &self,
        headers: &HeaderMap,
        request: gerax_mcp::JsonRpcRequest,
    ) -> HttpOutcome {
        let is_initialize = request.method == INITIALIZE;

        if let Err(error) = self.require_session_for_request(headers, is_initialize) {
            return error;
        }

        let response = McpDispatcher::new(self.server.clone())
            .dispatch(JsonRpcMessage::Request(request))
            .await
            .expect("request sempre gera resposta");

        if is_initialize && response.error.is_none() {
            self.sessions.create();
        }

        let session_id = self.sessions.current();
        let session = session_id.clone().unwrap_or_default();
        let body = serde_json::to_value(response).expect("JsonRpcResponse serializa");

        if accepts_sse(headers) && self.config.allow_sse {
            let sequence = self.next_sequence();
            let mut events = vec![prime_event(&session, sequence)];
            events.extend(event_sequence(&session, &[body], sequence));
            let body = events.iter().map(SseEvent::to_body).collect::<String>();
            HttpOutcome::Sse { body, session_id }
        } else {
            HttpOutcome::Json {
                status: StatusCode::OK,
                body,
                session_id,
            }
        }
    }

    fn require_session(&self, headers: &HeaderMap) -> Result<Option<SessionId>, HttpOutcome> {
        match headers.get(MCP_SESSION_HEADER) {
            None => Err(transport_error(
                StatusCode::BAD_REQUEST,
                -32600,
                "MCP-Session-Id ausente",
            )),
            Some(value) => {
                let id = value.to_str().unwrap_or_default().to_owned();
                if self.sessions.matches(&id) {
                    Ok(Some(id))
                } else {
                    Err(transport_error(
                        StatusCode::NOT_FOUND,
                        -32600,
                        "Sessão desconhecida ou expirada",
                    ))
                }
            }
        }
    }

    fn require_session_for_request(
        &self,
        headers: &HeaderMap,
        is_initialize: bool,
    ) -> Result<(), HttpOutcome> {
        let header = headers
            .get(MCP_SESSION_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        if is_initialize {
            if let (Some(current), Some(received)) = (self.sessions.current(), header)
                && received != current
            {
                return Err(transport_error(
                    StatusCode::NOT_FOUND,
                    -32600,
                    "Sessão desconhecida ou expirada",
                ));
            }
            Ok(())
        } else {
            match header {
                None => Err(transport_error(
                    StatusCode::BAD_REQUEST,
                    -32600,
                    "MCP-Session-Id ausente",
                )),
                Some(id) if self.sessions.matches(&id) => Ok(()),
                Some(_) => Err(transport_error(
                    StatusCode::NOT_FOUND,
                    -32600,
                    "Sessão desconhecida ou expirada",
                )),
            }
        }
    }

    fn validate_headers(&self, headers: &HeaderMap) -> Result<(), HttpOutcome> {
        validate_origin(&self.config.origin_policy, headers)?;
        validate_protocol_version(&self.config.protocol_versions, headers)?;
        Ok(())
    }

    fn next_sequence(&self) -> u64 {
        self.event_sequence.fetch_add(1, Ordering::Relaxed)
    }
}

/// Builder para o [`McpHttpServer`].
#[derive(Default)]
pub struct McpHttpServerBuilder {
    server: Option<Arc<McpServer>>,
    config: HttpTransportConfig,
}

impl McpHttpServerBuilder {
    /// Define o servidor MCP.
    pub fn server(mut self, server: impl Into<Arc<McpServer>>) -> Self {
        self.server = Some(server.into());
        self
    }

    /// Define o caminho do endpoint MCP (deve começar com `/`).
    pub fn endpoint(mut self, path: impl Into<String>) -> Self {
        self.config.endpoint = path.into();
        self
    }

    /// Define a política de `Origin`.
    pub fn origin_policy(mut self, policy: OriginPolicy) -> Self {
        self.config.origin_policy = policy;
        self
    }

    /// Habilita/desabilita respostas SSE a requests POST.
    pub fn allow_sse(mut self, allow: bool) -> Self {
        self.config.allow_sse = allow;
        self
    }

    /// Habilita/desabilita o stream SSE iniciado pelo cliente (GET).
    pub fn allow_client_stream(mut self, allow: bool) -> Self {
        self.config.allow_client_stream = allow;
        self
    }

    /// Habilita/desabilita o encerramento de sessão via DELETE.
    pub fn allow_session_delete(mut self, allow: bool) -> Self {
        self.config.allow_session_delete = allow;
        self
    }

    /// Define as versões de protocolo aceitas no header `MCP-Protocol-Version`.
    pub fn protocol_versions(mut self, versions: Vec<String>) -> Self {
        self.config.protocol_versions = versions;
        self
    }

    /// Constrói o [`McpHttpServer`].
    ///
    /// # Panics
    ///
    /// Pânico se nenhum servidor foi informado ou se o endpoint não
    /// começa com `/`.
    pub fn build(self) -> McpHttpServer {
        let server = self
            .server
            .expect("McpHttpServer exige um McpServer (use for_server ou .server)");
        if !self.config.endpoint.starts_with('/') {
            panic!(
                "endpoint MCP deve começar com '/': {}",
                self.config.endpoint
            );
        }
        McpHttpServer {
            server,
            config: Arc::new(self.config),
            sessions: Arc::new(SessionStore::new()),
            event_sequence: Arc::new(AtomicU64::new(1)),
        }
    }
}

impl std::fmt::Debug for McpHttpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpHttpServer")
            .field("server", &self.server.name())
            .field("endpoint", &self.config.endpoint)
            .field("session_id", &self.sessions.current())
            .finish()
    }
}

// --- Helpers de validação ---

/// Verifica `Origin` conforme a política configurada.
fn validate_origin(policy: &OriginPolicy, headers: &HeaderMap) -> Result<(), HttpOutcome> {
    let Some(origin) = headers.get(ORIGIN) else {
        return Ok(());
    };
    let origin = origin.to_str().unwrap_or_default().to_owned();
    let allowed = match policy {
        OriginPolicy::AllowAll => true,
        OriginPolicy::Allowlist(allowed) => allowed.contains(&origin),
        OriginPolicy::DenyIfPresent => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(transport_error(
            StatusCode::FORBIDDEN,
            -32600,
            "Origin não autorizado",
        ))
    }
}

/// Verifica o header `MCP-Protocol-Version`, quando presente.
fn validate_protocol_version(supported: &[String], headers: &HeaderMap) -> Result<(), HttpOutcome> {
    let Some(value) = headers.get(MCP_PROTOCOL_VERSION_HEADER) else {
        return Ok(());
    };
    let version = value.to_str().unwrap_or_default();
    if supported.iter().any(|supported| supported == version) {
        Ok(())
    } else {
        Err(transport_error(
            StatusCode::BAD_REQUEST,
            -32600,
            format!("MCP-Protocol-Version não suportado: {version}"),
        ))
    }
}

fn accepts_sse(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/event-stream"))
        .unwrap_or(false)
}

/// Deserializa o corpo em um [`JsonRpcMessage`].
fn parse_message(body: &Bytes) -> Result<JsonRpcMessage, HttpOutcome> {
    let text = std::str::from_utf8(body)
        .map_err(|_| transport_error(StatusCode::BAD_REQUEST, -32700, "Corpo não é UTF-8"))?;
    let value: Value = serde_json::from_str(text)
        .map_err(|_| transport_error(StatusCode::BAD_REQUEST, -32700, "Parse error"))?;
    serde_json::from_value::<JsonRpcMessage>(value).map_err(|_| {
        transport_error(
            StatusCode::BAD_REQUEST,
            -32600,
            "Mensagem JSON-RPC inválida",
        )
    })
}

fn transport_error(status: StatusCode, code: i32, message: impl Into<String>) -> HttpOutcome {
    HttpOutcome::Error {
        status,
        code,
        message: message.into(),
    }
}

// --- Handlers axum ---

async fn post_handler(
    State(state): State<McpHttpServer>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let body = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .unwrap_or_default();
    outcome_to_response(state.handle_post(&headers, body).await)
}

async fn get_handler(State(state): State<McpHttpServer>, headers: HeaderMap) -> Response {
    match state.handle_get(&headers).await {
        HttpOutcome::Sse { session_id, .. } => match session_id {
            Some(session) => {
                let mut response = notification_stream_response(&state, &session);
                insert_session(&mut response, &session);
                response
            }
            None => StatusCode::NO_CONTENT.into_response(),
        },
        outcome => outcome_to_response(outcome),
    }
}

async fn delete_handler(State(state): State<McpHttpServer>, headers: HeaderMap) -> Response {
    outcome_to_response(state.handle_delete(&headers).await)
}

fn outcome_to_response(outcome: HttpOutcome) -> Response {
    match outcome {
        HttpOutcome::Json {
            status,
            body,
            session_id,
        } => {
            let mut response = (status, body.to_string()).into_response();
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            if let Some(id) = session_id {
                insert_session(&mut response, &id);
            }
            response
        }
        HttpOutcome::Accepted { session_id } => {
            let mut response = StatusCode::ACCEPTED.into_response();
            if let Some(id) = session_id {
                insert_session(&mut response, &id);
            }
            response
        }
        HttpOutcome::Error {
            status,
            code,
            message,
        } => json_error_response(status, code, message),
        HttpOutcome::Sse { body, session_id } => {
            let mut response = (StatusCode::OK, body).into_response();
            response
                .headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
            if let Some(id) = session_id {
                insert_session(&mut response, &id);
            }
            response
        }
        HttpOutcome::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED.into_response(),
        HttpOutcome::Empty { status } => status.into_response(),
    }
}

fn json_error_response(status: StatusCode, code: i32, message: String) -> Response {
    let value = json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": { "code": code, "message": message }
    });
    let mut response = (status, value.to_string()).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

fn insert_session(response: &mut Response, id: &str) {
    if let Ok(value) = HeaderValue::from_str(id) {
        response.headers_mut().insert(MCP_SESSION_HEADER, value);
    }
}

/// Stream SSE que envia o prime event e depois as notifications do
/// [`McpServer`] até a stream ser encerrada pelo cliente.
fn notification_stream_response(server: &McpHttpServer, session_id: &str) -> Response {
    let sequence = server.next_sequence();
    let prime = Event::default()
        .id(format!("{session_id}:{sequence}"))
        .data("");

    let prefix = stream::iter(vec![Ok::<Event, Infallible>(prime)]);
    let receiver = server.server.notifications().subscribe();
    let notifications = stream::unfold(receiver, {
        let sequence = server.event_sequence.clone();
        let session_id = session_id.to_owned();
        move |mut receiver| {
            let sequence = sequence.clone();
            let session_id = session_id.clone();
            async move {
                loop {
                    match receiver.recv().await {
                        Ok(notification) => {
                            let id = format!(
                                "{session_id}:{}",
                                sequence.fetch_add(1, Ordering::Relaxed)
                            );
                            let value = serde_json::to_value(notification).ok()?;
                            let event = Event::default().id(id).data(value.to_string());
                            return Some((Ok::<Event, Infallible>(event), receiver));
                        }
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => return None,
                    }
                }
            }
        }
    });

    Sse::new(prefix.chain(notifications))
        .keep_alive(KeepAlive::default())
        .into_response()
}
