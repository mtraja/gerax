use gerax_core::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turma {
    pub id: Option<String>,
    pub nome: String,
    pub professor_id: String,
}

impl Entity for Turma {
    fn collection_name() -> &'static str {
        "turmas"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CriarTurma {
    pub nome: String,
    pub professor_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AtualizarTurma {
    pub nome: String,
    pub professor_id: String,
}
