//! Transportes do MCP.
//!
//! Um [`Transport`] separa o protocolo da comunicação (STDIO, HTTP, ...).
//! O transporte é bidirecional: o lado servidor envía [`JsonRpcMessage::Response`]
//! e notifications; o lado cliente envia [`JsonRpcMessage::Request`] e
//! notifications. Nem o [`crate::dispatcher::McpDispatcher`] nem o
//! [`crate::client::McpClient`] conhecem o transporte concreto, nem o
//! transporte conhece regras de negócio.
//!
//! ```text
//! Transport
//!    ↓
//! MCP Dispatcher / McpClient
//!    ↓
//! McpServer
//! ```

mod stdio;

pub use stdio::StdioTransport;

use async_trait::async_trait;

use crate::protocol::{JsonRpcMessage, JsonRpcNotification};

/// Erro de transporte.
///
/// O `Clone` é manual porque [`std::io::Error`] não implementa `Clone`.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// Falha de I/O.
    #[error("erro de I/O no transporte: {0}")]
    Io(#[from] std::io::Error),
    /// Mensagem recebida não é JSON-RPC válido.
    #[error("mensagem inválida no transporte: {0}")]
    InvalidMessage(String),
    /// Falha ao iniciar o processo do transporte STDIO.
    #[error("falha ao iniciar o processo do transporte: {0}")]
    ProcessSpawn(String),
    /// O processo do transporte STDIO terminou inesperadamente.
    #[error("o processo do transporte terminou com código {code}")]
    ProcessExited {
        /// Código de saída do processo (negativo se sinalizado).
        code: i32,
    },
}

impl Clone for TransportError {
    fn clone(&self) -> Self {
        match self {
            Self::Io(error) => Self::Io(std::io::Error::new(error.kind(), error.to_string())),
            Self::InvalidMessage(message) => Self::InvalidMessage(message.clone()),
            Self::ProcessSpawn(message) => Self::ProcessSpawn(message.clone()),
            Self::ProcessExited { code } => Self::ProcessExited { code: *code },
        }
    }
}

/// Abstração de transporte MCP.
///
/// Recebe mensagens do peer e envia mensagens/notifications. `Ok(None)`
/// em [`receive`](Self::receive) indica fim da stream (EOF) e encerra o
/// lado da comunicação que está lendo.
#[async_trait]
pub trait Transport: Send {
    /// Recebe a próxima mensagem do peer.
    ///
    /// Retorna `None` no fim da stream (EOF).
    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>, TransportError>;

    /// Envia uma mensagem JSON-RPC para o peer.
    ///
    /// O servidor envia [`JsonRpcMessage::Response`]; o cliente envia
    /// [`JsonRpcMessage::Request`].
    async fn send(&mut self, message: JsonRpcMessage) -> Result<(), TransportError>;

    /// Envia uma notification para o peer.
    async fn notify(&mut self, notification: JsonRpcNotification) -> Result<(), TransportError>;
}
