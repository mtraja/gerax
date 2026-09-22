use crate::adapters::inbound::http::handlers::{
    atualizar_aluno, atualizar_professor, atualizar_turma, criar_aluno, criar_matricula,
    criar_professor, criar_turma, deletar_aluno, deletar_matricula, deletar_professor,
    deletar_turma, listar_alunos, listar_alunos_por_turma, listar_matriculas, listar_professores,
    listar_turmas, obter_aluno, obter_matricula, obter_professor, obter_turma,
};
use crate::bootstrap::AppState;
use gerax_app::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(aluno_routes())
        .merge(professor_routes())
        .merge(turma_routes())
        .merge(matricula_routes())
}

fn aluno_routes() -> Router<AppState> {
    Router::new()
        .get("/alunos", listar_alunos)
        .post("/alunos", criar_aluno)
        .get("/alunos/:id", obter_aluno)
        .put("/alunos/:id", atualizar_aluno)
        .delete("/alunos/:id", deletar_aluno)
}

fn professor_routes() -> Router<AppState> {
    Router::new()
        .get("/professores", listar_professores)
        .post("/professores", criar_professor)
        .get("/professores/:id", obter_professor)
        .put("/professores/:id", atualizar_professor)
        .delete("/professores/:id", deletar_professor)
}

fn turma_routes() -> Router<AppState> {
    Router::new()
        .get("/turmas", listar_turmas)
        .post("/turmas", criar_turma)
        .get("/turmas/:id", obter_turma)
        .put("/turmas/:id", atualizar_turma)
        .delete("/turmas/:id", deletar_turma)
        .get("/turmas/:id/alunos", listar_alunos_por_turma)
}

fn matricula_routes() -> Router<AppState> {
    Router::new()
        .get("/matriculas", listar_matriculas)
        .post("/matriculas", criar_matricula)
        .get("/matriculas/:id", obter_matricula)
        .delete("/matriculas/:id", deletar_matricula)
}
