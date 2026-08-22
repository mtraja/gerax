use std::sync::Arc;

use gerax_app::{
    ActixRuntime, App, AppError, Connection, Context, HttpServerError, Router, ServerResult,
    DbError, PostgresConnection, PostgresRepository, Repository,
};
use gerax_core::Entity;
use gerax_http::routing::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Student {
    id: Option<String>,
    name: String,
    email: String,
}

impl Entity for Student {
    fn collection_name() -> &'static str {
        "students"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
struct CreateStudentInput {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct UpdateStudentInput {
    name: String,
    email: String,
}

struct AppState {
    db: Arc<PostgresConnection>,
}

fn students_repo(state: &Arc<AppState>) -> PostgresRepository<Student> {
    PostgresRepository::<Student>::new(state.db.clone())
}

fn db_err(err: DbError) -> HttpServerError {
    HttpServerError::HandlerError(err.to_string())
}

async fn list_students(ctx: Context<AppState>) -> ServerResult<Response> {
    let repo = students_repo(&ctx.state());
    let students = repo.find_all().await.map_err(db_err)?;

    let json = serde_json::to_vec(&students).map_err(|e| db_err(DbError::serialization(e)))?;
    Ok(Response::ok(json))
}

async fn get_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = ctx
        .params()
        .get("id")
        .ok_or_else(|| HttpServerError::HandlerError("missing id".into()))?
        .to_string();

    let repo = students_repo(&ctx.state());
    let student = repo.find_by_id(&id).await.map_err(db_err)?;

    match student {
        Some(s) => {
            let json = serde_json::to_vec(&s).map_err(|e| db_err(DbError::serialization(e)))?;
            Ok(Response::ok(json))
        }
        None => Ok(Response::not_found()),
    }
}

async fn create_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CreateStudentInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;

    let repo = students_repo(&ctx.state());
    let student = Student {
        id: None,
        name: input.name,
        email: input.email,
    };
    let created = repo.insert(student).await.map_err(db_err)?;

    let json = serde_json::to_vec(&created).map_err(|e| db_err(DbError::serialization(e)))?;
    Ok(Response::ok(json))
}

async fn update_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = ctx
        .params()
        .get("id")
        .ok_or_else(|| HttpServerError::HandlerError("missing id".into()))?
        .to_string();

    let input: UpdateStudentInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;

    let repo = students_repo(&ctx.state());
    let existing = repo
        .find_by_id(&id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| HttpServerError::HandlerError("student not found".into()))?;

    let student = Student {
        id: existing.id(),
        name: input.name,
        email: input.email,
    };
    repo.update(student).await.map_err(db_err)?;

    let updated = repo
        .find_by_id(&id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| HttpServerError::HandlerError("student not found after update".into()))?;

    let json = serde_json::to_vec(&updated).map_err(|e| db_err(DbError::serialization(e)))?;
    Ok(Response::ok(json))
}

async fn delete_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = ctx
        .params()
        .get("id")
        .ok_or_else(|| HttpServerError::HandlerError("missing id".into()))?
        .to_string();

    let repo = students_repo(&ctx.state());
    repo.delete(&id).await.map_err(db_err)?;

    Ok(Response::ok(b"{}".to_vec()))
}

fn router() -> Router<AppState> {
    Router::new()
        .get("/students", list_students)
        .post("/students", create_student)
        .get("/students/:id", get_student)
        .put("/students/:id", update_student)
        .delete("/students/:id", delete_student)
}

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    dotenv::dotenv().ok();

    let connection = Arc::new(PostgresConnection::connect().await?);
    connection.ping().await?;

    let students_table = PostgresRepository::<Student>::new(connection.clone());
    students_table.create_table().await?;

    let state = AppState { db: connection };

    App::builder(state)
        .router(router())
        .build()
        .run::<ActixRuntime>()
        .await
}
