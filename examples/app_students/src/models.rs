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
