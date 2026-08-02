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
//! use gerax_graphql::{Schema, GraphqlRequest, GraphqlResponse, GraphqlError};
//!
//! let schema = Schema::builder()
//!     .query(QueryRoot)
//!     .finish()?;
//! ```

pub mod schema;
pub mod request;
pub mod response;
pub mod error;

pub use schema::{Schema, SchemaBuilder};
pub use request::GraphqlRequest;
pub use response::GraphqlResponse;
pub use error::GraphqlError;