use gerax_app::{Context, DbError, HttpServerError, ServerResult};
use gerax_http::routing::Response;
use serde::Serialize;

use crate::models::{
    CreateClassInput, CreateEnrollmentInput, CreateStudentInput, CreateTeacherInput,
    UpdateClassInput, UpdateStudentInput, UpdateTeacherInput,
};
use crate::state::AppState;

fn db_err(err: DbError) -> HttpServerError {
    HttpServerError::HandlerError(err.to_string())
}

fn json_response(value: &impl Serialize) -> ServerResult<Response> {
    let json = serde_json::to_vec(value).map_err(|e| db_err(DbError::serialization(e)))?;
    Ok(Response::ok(json))
}

fn path_id<S>(ctx: &Context<S>) -> String {
    ctx.params()
        .get("id")
        .ok_or_else(|| HttpServerError::HandlerError("missing id".into()))
        .unwrap()
        .to_string()
}

pub async fn list_students(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().students.list().await.map_err(db_err)?;
    json_response(&items)
}

pub async fn get_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    match ctx.state().students.get(&id).await.map_err(db_err)? {
        Some(item) => json_response(&item),
        None => Ok(Response::not_found()),
    }
}

pub async fn create_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CreateStudentInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().students.create(input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn update_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    let input: UpdateStudentInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().students.update(&id, input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn delete_student(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    ctx.state().students.delete(&id).await.map_err(db_err)?;
    json_response(&serde_json::json!({}))
}

pub async fn list_teachers(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().teachers.list().await.map_err(db_err)?;
    json_response(&items)
}

pub async fn get_teacher(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    match ctx.state().teachers.get(&id).await.map_err(db_err)? {
        Some(item) => json_response(&item),
        None => Ok(Response::not_found()),
    }
}

pub async fn create_teacher(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CreateTeacherInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().teachers.create(input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn update_teacher(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    let input: UpdateTeacherInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().teachers.update(&id, input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn delete_teacher(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    ctx.state().teachers.delete(&id).await.map_err(db_err)?;
    json_response(&serde_json::json!({}))
}

pub async fn list_classes(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().classes.list().await.map_err(db_err)?;
    json_response(&items)
}

pub async fn get_class(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    match ctx.state().classes.get(&id).await.map_err(db_err)? {
        Some(item) => json_response(&item),
        None => Ok(Response::not_found()),
    }
}

pub async fn create_class(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CreateClassInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().classes.create(input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn update_class(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    let input: UpdateClassInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().classes.update(&id, input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn delete_class(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    ctx.state().classes.delete(&id).await.map_err(db_err)?;
    json_response(&serde_json::json!({}))
}

pub async fn list_enrollments(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().enrollments.list().await.map_err(db_err)?;
    json_response(&items)
}

pub async fn get_enrollment(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    match ctx.state().enrollments.get(&id).await.map_err(db_err)? {
        Some(item) => json_response(&item),
        None => Ok(Response::not_found()),
    }
}

pub async fn create_enrollment(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CreateEnrollmentInput =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().enrollments.create(input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn delete_enrollment(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    ctx.state().enrollments.delete(&id).await.map_err(db_err)?;
    json_response(&serde_json::json!({}))
}
