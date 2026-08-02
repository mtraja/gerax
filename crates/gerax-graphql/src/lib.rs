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
//! ## Exemplo futuro
//!
//! ```rust
//! use gerax_graphql::{GraphQLExecutor, SchemaBuilder};
//! use gerax_core::Entity;
//!
//! #[derive(Entity)]
//! struct User { ... }
//!
//! let schema = SchemaBuilder::new()
//!     .entity::<User>()
//!     .build();
//!
//! let executor = GraphQLExecutor::new(schema);
//! let result = executor.execute(query).await?;
//! ```

pub mod schema;
pub mod executor;
pub mod error;

pub use error::GraphQLError;
