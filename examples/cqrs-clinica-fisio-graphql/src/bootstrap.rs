//! Composition root: conexão com o banco, registro de handlers,
//! construção das buses, do schema GraphQL e do router HTTP.

use std::sync::Arc;

use gerax_cqrs::{CommandBus, HandlerRegistry, QueryBus, register_commands, register_queries};
use gerax_db::Connection;
use gerax_postgres::PostgresConnection;
use tokio::net::TcpListener;

use crate::adapters::inbound::graphql::{GraphqlSchema, router, schema};
use crate::adapters::outbound::postgres::{PostgresPacienteRepository, PostgresTerapiaRepository};
use crate::application::handlers::paciente::{
    AtualizarPacienteHandler, CriarPacienteHandler, DeletarPacienteHandler, ListarPacientesHandler,
    ObterPacienteHandler,
};
use crate::application::handlers::terapia::{
    AtualizarTerapiaHandler, CriarTerapiaHandler, DeletarTerapiaHandler,
    ListarTerapiasDoPacienteHandler, ListarTerapiasHandler, ObterTerapiaHandler,
};

const HOST: &str = "0.0.0.0:8080";

/// Estado compartilhado das rotas HTTP.
///
/// O schema GraphQL é compartilhado por `Arc` (clone por requisição
/// custa apenas um incremento de referência); os resolvers acessam as
/// buses através de `ctx.data::<AppState>()`.
#[derive(Clone)]
pub struct AppState {
    pub command_bus: CommandBus,
    pub query_bus: QueryBus,
    pub graphql_schema: Arc<GraphqlSchema>,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("gerax_postgres=debug,cqrs_clinica_fisio=info")
            }),
        )
        .try_init();

    let connection = Arc::new(PostgresConnection::connect().await?);
    connection.ping().await?;

    let paciente_repo = Arc::new(PostgresPacienteRepository::new(connection.clone()));
    paciente_repo.create_table().await?;

    let terapia_repo = Arc::new(PostgresTerapiaRepository::new(connection.clone()));
    terapia_repo.create_table().await?;

    let mut registry = HandlerRegistry::new();
    register_commands!(
        registry,
        CriarPacienteHandler::new(paciente_repo.clone()),
        AtualizarPacienteHandler::new(paciente_repo.clone()),
        DeletarPacienteHandler::new(paciente_repo.clone()),
        CriarTerapiaHandler::new(terapia_repo.clone(), paciente_repo.clone()),
        AtualizarTerapiaHandler::new(terapia_repo.clone()),
        DeletarTerapiaHandler::new(terapia_repo.clone()),
    );
    register_queries!(
        registry,
        ObterPacienteHandler::new(paciente_repo.clone()),
        ListarPacientesHandler::new(paciente_repo.clone()),
        ObterTerapiaHandler::new(terapia_repo.clone()),
        ListarTerapiasHandler::new(terapia_repo.clone()),
        ListarTerapiasDoPacienteHandler::new(terapia_repo.clone()),
    );

    let registry = Arc::new(registry);
    let state = AppState {
        command_bus: CommandBus::new(registry.clone()),
        query_bus: QueryBus::new(registry),
        graphql_schema: Arc::new(schema()),
    };

    let listener = TcpListener::bind(HOST).await?;
    tracing::info!("API GraphQL da clínica ouvindo em http://{HOST}/graphql");
    axum::serve(listener, router().with_state(state)).await?;

    Ok(())
}
