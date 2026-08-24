use gerax_core::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Professor {
    pub id: Option<String>,
    pub nome: String,
    pub email: String,
}

impl Entity for Professor {
    fn collection_name() -> &'static str {
        "professores"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CriarProfessor {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct AtualizarProfessor {
    pub nome: String,
    pub email: String,
}
