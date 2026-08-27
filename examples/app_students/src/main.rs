mod handlers;
mod models;
mod router;
mod services;
mod state;

use std::sync::Arc;

use gerax_app::{ActixRuntime, App, AppError, Connection, PostgresConnection, PostgresRepository};
use gerax_http::CorsConfig;

use crate::models::{Aluno, Matricula, Professor, Turma};
use crate::router::router;
use crate::state::AppState;

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("gerax_postgres=debug,app_students=info"))
        )
        .try_init();

    let connection = Arc::new(PostgresConnection::connect().await?);
    connection.ping().await?;

    PostgresRepository::<Aluno>::new(connection.clone())
        .create_table()
        .await?;
    PostgresRepository::<Professor>::new(connection.clone())
        .create_table()
        .await?;
    PostgresRepository::<Turma>::new(connection.clone())
        .create_table()
        .await?;
    PostgresRepository::<Matricula>::new(connection.clone())
        .create_table()
        .await?;

    let state = AppState::new(connection);

    let cors = CorsConfig::new()
    .allowed_origin("http://localhost:5173");

    App::builder(state)
        .router(router())
        .cors(cors)
        .build()
        .run::<ActixRuntime>()
        .await
}
