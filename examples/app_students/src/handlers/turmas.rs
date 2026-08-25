use gerax_app::{Context, DbError, ServerResult};
use gerax_http::routing::Response;

use crate::models::{AtualizarTurma, CriarTurma};
use crate::state::AppState;

use super::utils::{db_err, json_response, path_id};

pub async fn listar(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().turmas.listar().await.map_err(db_err)?;
    json_response(&items)
}

pub async fn obter(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    match ctx.state().turmas.obter(&id).await.map_err(db_err)? {
        Some(item) => json_response(&item),
        None => Ok(Response::not_found()),
    }
}

pub async fn criar(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CriarTurma =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().turmas.criar(input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn atualizar(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    let input: AtualizarTurma =
        serde_json::from_slice(&ctx.request().body).map_err(|e| db_err(DbError::serialization(e)))?;
    let item = ctx.state().turmas.atualizar(&id, input).await.map_err(db_err)?;
    json_response(&item)
}

pub async fn deletar(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    ctx.state().turmas.deletar(&id).await.map_err(db_err)?;
    json_response(&serde_json::json!({}))
}

pub async fn alunos(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx);
    let items = ctx
        .state()
        .matriculas
        .listar_alunos_por_turma(&id)
        .await
        .map_err(db_err)?;
    json_response(&items)
}
