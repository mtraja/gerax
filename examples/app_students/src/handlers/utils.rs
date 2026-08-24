use gerax_app::{Context, DbError, HttpServerError, ServerResult};
use gerax_http::routing::Response;
use serde::Serialize;

pub fn db_err(err: DbError) -> HttpServerError {
    HttpServerError::HandlerError(err.to_string())
}

pub fn json_response(value: &impl Serialize) -> ServerResult<Response> {
    let json = serde_json::to_vec(value).map_err(|e| db_err(DbError::serialization(e)))?;
    Ok(Response::ok(json))
}

pub fn path_id<S>(ctx: &Context<S>) -> String {
    ctx.params()
        .get("id")
        .ok_or_else(|| HttpServerError::HandlerError("identificador ausente".into()))
        .unwrap()
        .to_string()
}
