mod aluno;
mod matricula;
mod professor;
mod turma;

use crate::state::AppState;
use gerax_app::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(aluno::routes())
        .merge(professor::routes())
        .merge(turma::routes())
        .merge(matricula::routes())
}
