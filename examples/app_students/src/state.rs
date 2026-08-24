use std::sync::Arc;

use gerax_app::PostgresConnection;

use crate::services::{ServicoAluno, ServicoMatricula, ServicoProfessor, ServicoTurma};

pub struct AppState {
    pub alunos: Arc<ServicoAluno>,
    pub professores: Arc<ServicoProfessor>,
    pub turmas: Arc<ServicoTurma>,
    pub matriculas: Arc<ServicoMatricula>,
}

impl AppState {
    pub fn new(db: Arc<PostgresConnection>) -> Self {
        Self {
            alunos: Arc::new(ServicoAluno::new(db.clone())),
            professores: Arc::new(ServicoProfessor::new(db.clone())),
            turmas: Arc::new(ServicoTurma::new(db.clone())),
            matriculas: Arc::new(ServicoMatricula::new(db)),
        }
    }
}
