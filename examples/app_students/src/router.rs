use crate::handlers::{
    alunos, matriculas, professores, turmas,
};
use crate::state::AppState;
use gerax_app::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .get("/alunos", alunos::listar)
        .post("/alunos", alunos::criar)
        .get("/alunos/:id", alunos::obter)
        .put("/alunos/:id", alunos::atualizar)
        .delete("/alunos/:id", alunos::deletar)
        .get("/professores", professores::listar)
        .post("/professores", professores::criar)
        .get("/professores/:id", professores::obter)
        .put("/professores/:id", professores::atualizar)
        .delete("/professores/:id", professores::deletar)
        .get("/turmas", turmas::listar)
        .post("/turmas", turmas::criar)
        .get("/turmas/:id", turmas::obter)
        .put("/turmas/:id", turmas::atualizar)
        .delete("/turmas/:id", turmas::deletar)
        .get("/matriculas", matriculas::listar)
        .post("/matriculas", matriculas::criar)
        .get("/matriculas/:id", matriculas::obter)
        .delete("/matriculas/:id", matriculas::deletar)
}
