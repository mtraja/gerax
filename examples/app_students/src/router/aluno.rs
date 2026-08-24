use crate::handlers::alunos::{
    atualizar, criar, deletar, listar, obter,
};
use crate::state::AppState;
use gerax_app::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .get("/alunos", listar)
        .post("/alunos", criar)
        .get("/alunos/:id", obter)
        .put("/alunos/:id", atualizar)
        .delete("/alunos/:id", deletar)
}
