use gerax_core::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aluno {
    pub id: Option<String>,
    pub nome: String,
    pub email: String,
}

impl Entity for Aluno {
    fn collection_name() -> &'static str {
        "alunos"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CriarAluno {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct AtualizarAluno {
    pub nome: String,
    pub email: String,
}
