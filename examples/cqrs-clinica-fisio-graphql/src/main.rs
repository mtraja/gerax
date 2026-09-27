//! # cqrs-clinica-fisio-graphql
//!
//! Exemplo de integração do `gerax-cqrs` em uma aplicação com
//! arquitetura hexagonal: domínio de clínica de fisioterapia com as
//! entidades `Paciente` e `Terapia`, persistência em PostgreSQL
//! (`gerax-postgres`) e API GraphQL servida por axum com o
//! `gerax-graphql`.
//!
//! Fluxo:
//!
//! ```text
//! GraphQL (axum) → Schema gerax-graphql → CommandBus/QueryBus
//!              → CommandHandler/QueryHandler
//!              → PacienteRepository/TerapiaRepository (porta)
//!              → Postgres*Repository
//! ```

mod adapters;
mod application;
mod bootstrap;
mod domain;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    bootstrap::run().await
}
