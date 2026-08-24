mod handlers;
mod models;
mod router;
mod services;
mod state;

use std::sync::Arc;

use gerax_app::{ActixRuntime, App, AppError, Connection, PostgresConnection, PostgresRepository};

use crate::models::{Aluno, Matricula, Professor, Turma};
use crate::router::router;
use crate::state::AppState;

#[actix_web::main]
async fn main() -> Result<(), AppError> {
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

    App::builder(state)
        .router(router())
        .build()
        .run::<ActixRuntime>()
        .await
}
