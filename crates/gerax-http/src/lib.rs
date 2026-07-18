//! # gerax-http
//!
//! Crate para abstrações HTTP do framework Gerax.
//!
//! Esta crate define a interface abstrata para frameworks HTTP, sem
//! dependências de implementação concreta. A aplicação escolhe a tecnologia
//! HTTP (Actix Web, Axum, Poem, etc.) e implementa os traits definidos aqui.
//!
//! - `HttpServer<S>` — trait do servidor HTTP
//! - `HttpServerBuilder<S>` — trait para construção via builder pattern
//! - `GeraxHttpError` — hierarquia de erros própria

pub mod builder;
pub mod error;
pub mod server;

pub use builder::HttpServerBuilder;
pub use error::GeraxHttpError;
pub use server::HttpServer;
