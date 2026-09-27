//! Adaptador de entrada via API GraphQL (axum + gerax-graphql).
//!
//! As raízes [`QueryRoot`] e [`MutationRoot`] despacham comandos e
//! consultas pelas buses (gerax-cqrs); o schema é construído com
//! [`gerax_graphql::Schema`] e executado pelo handle de axum.

mod dto;
mod error;
mod resolvers;
mod routes;

pub use resolvers::{MutationRoot, QueryRoot};
pub use routes::router;

use gerax_graphql::Schema;

/// Schema GraphQL com as raízes de query e mutation da clínica.
pub type GraphqlSchema = Schema<QueryRoot, MutationRoot, async_graphql::EmptySubscription>;

/// Constrói o schema GraphQL da aplicação.
pub fn schema() -> GraphqlSchema {
    Schema::builder()
        .query(QueryRoot)
        .mutation(MutationRoot)
        .finish()
        .expect("falha ao construir o schema GraphQL")
}
