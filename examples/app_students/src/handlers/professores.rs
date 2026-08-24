use gerax_app::{Context, DbError, ServerResult};
use gerax_http::routing::Response;

use crate::models::{AtualizarProfessor, CriarProfessor};
use crate::state::AppState;

use super::utils::{db_err, json_response, path_id};

pub async fn listar(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().professores.listar().await.map_err(db_err)?;
    json_response(&items)
}

pub async fn obter(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    match ctx.state().professores.obter(&id).await.map_err(db_err)? {
        Some(item) => json_response(&item),
        None => Ok(Response::not_found()),
    }
}

pub async fn criar(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CriarProfessor =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().professores.criar(input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn atualizar(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    let input: AtualizarProfessor =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().professores.atualizar(&id, input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn deletar(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    ctx.state().professores.deletar(&id).await.map_err(db_err)?;
    json_response(&serde_json::json!({}))
}
