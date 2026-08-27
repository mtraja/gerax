use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aluno {
    pub id: Option<String>,
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Professor {
    pub id: Option<String>,
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turma {
    pub id: Option<String>,
    pub nome: String,
    pub professor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matricula {
    pub id: Option<String>,
    pub aluno_id: String,
    pub turma_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    CampoObrigatorio(&'static str),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CampoObrigatorio(campo) => write!(f, "campo obrigatório: {campo}"),
        }
    }
}

impl std::error::Error for DomainError {}

fn texto(campo: &'static str, valor: String) -> Result<String, DomainError> {
    let valor = valor.trim().to_owned();
    if valor.is_empty() {
        Err(DomainError::CampoObrigatorio(campo))
    } else {
        Ok(valor)
    }
}

impl Aluno {
    pub fn novo(nome: String, email: String) -> Result<Self, DomainError> {
        Ok(Self {
            id: None,
            nome: texto("nome", nome)?,
            email: texto("email", email)?,
        })
    }
    pub fn atualizar(&mut self, nome: String, email: String) -> Result<(), DomainError> {
        self.nome = texto("nome", nome)?;
        self.email = texto("email", email)?;
        Ok(())
    }
}
impl Professor {
    pub fn novo(nome: String, email: String) -> Result<Self, DomainError> {
        Ok(Self {
            id: None,
            nome: texto("nome", nome)?,
            email: texto("email", email)?,
        })
    }
    pub fn atualizar(&mut self, nome: String, email: String) -> Result<(), DomainError> {
        self.nome = texto("nome", nome)?;
        self.email = texto("email", email)?;
        Ok(())
    }
}
impl Turma {
    pub fn nova(nome: String, professor_id: String) -> Result<Self, DomainError> {
        Ok(Self {
            id: None,
            nome: texto("nome", nome)?,
            professor_id: texto("professor_id", professor_id)?,
        })
    }
    pub fn atualizar(&mut self, nome: String, professor_id: String) -> Result<(), DomainError> {
        self.nome = texto("nome", nome)?;
        self.professor_id = texto("professor_id", professor_id)?;
        Ok(())
    }
}
impl Matricula {
    pub fn nova(aluno_id: String, turma_id: String) -> Result<Self, DomainError> {
        Ok(Self {
            id: None,
            aluno_id: texto("aluno_id", aluno_id)?,
            turma_id: texto("turma_id", turma_id)?,
        })
    }
}
