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
//! ```ignore
//! use gerax_graphql::{Schema, GraphqlRequest, GraphqlResponse, GraphqlError};
//!
//! let schema = Schema::builder()
//!     .query(QueryRoot)
//!     .finish()?;
//! ```

pub mod auth;
pub mod context;
pub mod dataloader;
pub mod error;
pub mod executor;
pub mod extensions;
pub mod handler;
pub mod middleware;
pub mod request;
pub mod resolver;
pub mod response;
pub mod schema;
pub mod subscription;
pub mod ui;

pub use auth::{AuthClaims, AuthExtractor, AuthIntegration, GraphqlAuthContext};
pub use context::{GraphqlContext, GraphqlExtensions};
pub use dataloader::{BatchLoader, DataLoader};
pub use error::GraphqlError;
pub use executor::{DefaultExecutor, Executor};
pub use extensions::cache::ResponseCache;
pub use extensions::complexity_depth::{ComplexityLimiter, DepthLimiter};
pub use extensions::introspection::IntrospectionController;
pub use extensions::persisted_query::{PersistedQueryCache, PersistedQueryManager};
pub use extensions::validation::{ArgumentsValidator, QueryValidator, SchemaValidator, Validator};
pub use handler::GraphqlHandler;
pub use middleware::{
    AuthMiddleware, CacheMiddleware, GraphqlMiddleware, LoggingMiddleware, MetricsMiddleware,
};
pub use request::GraphqlRequest;
pub use resolver::{MutationResolver, QueryResolver, Resolver, SubscriptionResolver};
pub use response::GraphqlResponse;
pub use schema::{Schema, SchemaBuilder};
pub use subscription::{Subscription, SubscriptionManager, WebSocketSubscriptionAdapter};
pub use ui::{GraphiQL, Playground};
