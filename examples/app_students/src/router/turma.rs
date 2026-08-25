use crate::handlers::turmas::{
    alunos, atualizar, criar, deletar, listar, obter,
};
use crate::state::AppState;
use gerax_app::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .get("/turmas", listar)
        .post("/turmas", criar)
        .get("/turmas/:id", obter)
        .put("/turmas/:id", atualizar)
        .delete("/turmas/:id", deletar)
        .get("/turmas/:id/alunos", alunos)
}
