//! # gerax-graphql
//!
//! Suporte a GraphQL no ecossistema Gerax.
//!
//! Este crate fornece:
//! - Construção de schema GraphQL a partir de entidades `gerax-core`
//! - Execução de queries e mutations
//! - Integração com `gerax-db` para resolver dados
//! - Integração com `gerax-http` para expor endpoint GraphQL
//!
//! ## Exemplo
//!
//! ```rust
//! use gerax_graphql::{GraphqlRequest, GraphqlResponse, GraphqlError};
//!
//! let request = GraphqlRequest::new("{ health }");
//! let response = GraphqlResponse::ok(serde_json::json!({"health": "ok"}));
//! ```

pub mod request;
pub mod response;
pub mod error;

pub use request::GraphqlRequest;
pub use response::GraphqlResponse;
pub use error::GraphqlError;