use gerax_core::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Matricula {
    pub id: Option<String>,
    pub aluno_id: String,
    pub turma_id: String,
}

impl Entity for Matricula {
    fn collection_name() -> &'static str {
        "matriculas"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CriarMatricula {
    pub aluno_id: String,
    pub turma_id: String,
}
