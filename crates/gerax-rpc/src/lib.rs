//! # gerax-rpc
//!
//! Abstrações RPC protocol-agnósticas para o ecossistema Gerax.
//!
//! Este crate define os contratos, modelos e abstrações para RPC
//! reutilizadas por todas as implementações do Gerax, incluindo
//! `gerax-grpc` e `gerax-capnp`.
//!
//! ## Abstrações
//!
//! - `RpcRequest<T>` - Requisição RPC genérica
//! - `RpcResponse<T>` - Resposta RPC genérica
//! - `RpcStatus` - Status de resposta RPC
//! - `RpcMetadata` - Metadados RPC
//! - `RpcContext` - Contexto de execução RPC
//! - `RpcExtensions` - Extensões opcionais
//! - `RpcMethod` - Método RPC
//! - `RpcService` - Serviço RPC
//! - `RpcServer` - Servidor RPC
//! - `RpcClient` - Cliente RPC
//! - `RpcTransport` - Transporte RPC
//! - `RpcStream` - Stream RPC
//! - `RpcError` - Erros RPC
//!
//! ## Dependências
//!
//! - `gerax-core` - Entidades e tipos base
//!
//! Não depende de:
//! - `tonic`, `prost` (gRPC)
//! - `capnp` (Cap'n Proto)
//! - `hyper`, `http` (HTTP)
//! - `tokio` (runtime assíncrono)
//! - `serde` (serialização)

pub mod client;
pub mod context;
pub mod error;
pub mod extensions;
pub mod metadata;
pub mod method;
pub mod request;
pub mod response;
pub mod server;
pub mod service;
pub mod status;
pub mod stream;
pub mod transport;

pub use client::{RpcClient, TransportRpcClient};
pub use context::{RpcContext, RpcContextBuilder};
pub use error::RpcError;
pub use extensions::RpcExtensions;
pub use metadata::{RpcMetadata, RpcMetadataBuilder};
pub use method::RpcMethod;
pub use request::{RpcRequest, RpcRequestBuilder};
pub use response::{RpcResponse, RpcResponseBuilder};
pub use server::{RpcServer, TransportRpcServer};
pub use service::{RpcService, RpcServiceBuilder, SimpleRpcService};
pub use status::RpcStatus;
pub use stream::{RpcResponseStream, RpcStream};
pub use transport::RpcTransport;
