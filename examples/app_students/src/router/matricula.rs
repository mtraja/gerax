use crate::handlers::matriculas::{
    criar, deletar, listar, obter,
};
use crate::state::AppState;
use gerax_app::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .get("/matriculas", listar)
        .post("/matriculas", criar)
        .get("/matriculas/:id", obter)
        .delete("/matriculas/:id", deletar)
}
