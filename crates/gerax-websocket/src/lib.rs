//! # gerax-websocket
//!
//! Suporte a WebSocket no ecossistema Gerax.
//!
//! Este crate fornece:
//! - Servidor WebSocket genérico
//! - Cliente WebSocket genérico
//! - Integração com `gerax-db` para resolver dados das entidades
//! - Integração com `gerax-http` para expor endpoints WebSocket

pub mod websocket;

pub use websocket::client::WebSocketClient;
pub use websocket::handler::{ServerError, WsContext, WsHandler, WsResult};
pub use websocket::message::WsMessage;
pub use websocket::repository::{RepositoryResolver, WsRepository};
pub use websocket::upgrade::WsUpgradeHandler;
pub use websocket::websocket::WebSocketServer;