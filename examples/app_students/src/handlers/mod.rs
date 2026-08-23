use gerax_app::{Context, DbError, HttpServerError, ServerResult};
use gerax_http::routing::Response;
use serde::Serialize;

use crate::models::{CreateStudentInput, UpdateStudentInput};
use crate::state::AppState;

fn db_err(err: DbError) -> HttpServerError {
    HttpServerError::HandlerError(err.to_string())
}

fn json_response(value: &impl Serialize) -> ServerResult<Response> {
    let json = serde_json::to_vec(value).map_err(|e| db_err(DbError::serialization(e)))?;
    Ok(Response::ok(json))
}

pub async fn list_students(ctx: Context<AppState>) -> ServerResult<Response> {
    let service = ctx.state().service.clone();
    let students = service.list().await.map_err(db_err)?;
    json_response(&students)
}

pub async fn get_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);

    let service = ctx.state().service.clone();
    match service.get(&id).await.map_err(db_err)? {
        Some(student) => json_response(&student),
        None => Ok(Response::not_found()),
    }
}

pub async fn create_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CreateStudentInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;

    let service = ctx.state().service.clone();
    let student = service.create(input).await.map_err(db_err)?;

    json_response(&student)
}

pub async fn update_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);

    let input: UpdateStudentInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;

    let service = ctx.state().service.clone();
    let student = service.update(&id, input).await.map_err(db_err)?;

    json_response(&student)
}

pub async fn delete_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);

    let service = ctx.state().service.clone();
    service.delete(&id).await.map_err(db_err)?;

    Ok(Response::ok(b"{}".to_vec()))
}

fn path_id<S>(ctx: &Context<S>) -> String {
    ctx.params()
        .get("id")
        .ok_or_else(|| HttpServerError::HandlerError("missing id".into()))
        .unwrap()
        .to_string()
}
