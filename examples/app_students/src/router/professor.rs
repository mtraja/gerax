use crate::handlers::professores::{
    atualizar, criar, deletar, listar, obter,
};
use crate::state::AppState;
use gerax_app::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .get("/professores", listar)
        .post("/professores", criar)
        .get("/professores/:id", obter)
        .put("/professores/:id", atualizar)
        .delete("/professores/:id", deletar)
}
