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
//! use gerax_grpc::GrpcServer;
//! # Ok::<(), gerax_grpc::GrpcError>(())
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod handler;
pub mod method;
pub mod proto;
pub mod server;
pub mod transport;

pub use client::GrpcClient;
pub use config::GrpcConfig;
pub use error::GrpcError;
pub use handler::GrpcHandler;
pub use method::GrpcService;
pub use proto::{ProtoSchema, builder::ProtoBuilder};
pub use server::GrpcServer;
pub use transport::{
    GrpcCodecHandler, GrpcCodecInvoker, GrpcInvoker, GrpcRequestHandler, TonicClientTransport,
    TonicServerConnection, TonicServerTransport,
};
