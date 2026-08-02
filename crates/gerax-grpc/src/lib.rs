//! # gerax-grpc
//!
//! Suporte a gRPC no ecossistema Gerax.
//!
//! Este crate fornece:
//! - Geração de schemas Protobuf a partir de entidades `gerax-core`
//! - Servidor gRPC genérico
//! - Cliente gRPC genérico
//! - Integração com `gerax-db` para resolver dados
//! - Integração com `gerax-http` para expor endpoints gRPC
//!
//! ## Exemplo futuro
//!
//! ```rust
//! use gerax_grpc::{GrpcServer, ProtoBuilder};
//! use gerax_core::Entity;
//!
//! #[derive(Entity)]
//! struct User { ... }
//!
//! let server = GrpcServer::<User>::new();
//! server.serve("[::1]:50051").await?;
//! ```

pub mod proto;
pub mod server;
pub mod client;
pub mod error;

pub use error::GrpcError;
