//! # gerax-mcp-http
//!
//! Transporte **Streamable HTTP** do [Model Context Protocol] para o
//! Gerax, conforme a especificação `2025-11-25`.
//!
//! A lógica MCP permanece na crate `gerax-mcp` ([`gerax_mcp::McpServer`]);
//! esta crate adiciona apenas o enquadramento HTTP: sessão, aplicação de
//! regras de `Origin`/`MCP-Protocol-Version`, respostas `application/json`
//! ou `text/event-stream` (SSE) e o roteamento axum.
//!
//! [Model Context Protocol]: https://modelcontextprotocol.io
//!
//! ```text
//! HTTP Client
//!      │
//!     POST/GET/DELETE /mcp
//!      │
//!      ▼
//! gerax-mcp-http (sessão, SSE, Origin, versão)
//!      │
//!      ▼
//! McpDispatcher ← McpServer (gerax-mcp)
//! ```
//!
//! # Uso com axum
//!
//! ```no_run
//! use gerax_mcp::McpServer;
//! use gerax_mcp_http::McpHttpServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServer::builder().name("escola").version("0.1.0").build();
//!
//!     let app = McpHttpServer::for_server(server)
//!         .endpoint("/mcp")
//!         .build()
//!         .router();
//!
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```

#![warn(missing_docs)]

pub mod config;
pub mod server;
pub mod session;
pub mod sse;

pub use config::{HttpTransportConfig, OriginPolicy};
pub use server::{HttpOutcome, McpHttpServer, McpHttpServerBuilder};
pub use session::{SessionId, SessionStore};
pub use sse::{SseEvent, event_sequence, prime_event};
