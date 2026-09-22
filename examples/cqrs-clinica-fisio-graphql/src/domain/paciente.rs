use serde::{Deserialize, Serialize};

use super::error::DomainError;
use super::value_objects::{Cpf, DataNascimento, Email, Nome, PacienteId, Telefone};

/// Entidade `Paciente`.
///
/// `id` é gerado pela infraestrutura de persistência (`None` antes do
/// primeiro `save`/`create`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paciente {
    pub id: Option<PacienteId>,
    pub nome: Nome,
    pub cpf: Cpf,
    pub data_nascimento: DataNascimento,
    pub telefone: Telefone,
    pub email: Email,
}

impl Paciente {
    pub fn novo(
        nome: String,
        cpf: String,
        data_nascimento: String,
        telefone: String,
        email: String,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: None,
            nome: Nome::new(nome)?,
            cpf: Cpf::new(cpf)?,
            data_nascimento: DataNascimento::new(data_nascimento)?,
            telefone: Telefone::new(telefone)?,
            email: Email::new(email)?,
        })
    }

    pub fn atualizar(&mut self, nome: String, telefone: String, email: String) -> Result<(), DomainError> {
        self.nome = Nome::new(nome)?;
        self.telefone = Telefone::new(telefone)?;
        self.email = Email::new(email)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemplo() -> Paciente {
        Paciente::novo(
            "Maria Silva".into(),
            "12345678900".into(),
            "1990-05-04".into(),
            "(85) 99999-0000".into(),
            "maria@email.com".into(),
        )
        .unwrap()
    }

    #[test]
    fn novo_cria_paciente_sem_id() {
        let paciente = exemplo();
        assert_eq!(paciente.id, None);
        assert_eq!(paciente.nome.as_str(), "Maria Silva");
    }

    #[test]
    fn novo_remove_espacos_em_branco() {
        let paciente = Paciente::novo(
            "  Maria  ".into(),
            " 12345678900 ".into(),
            "1990-05-04".into(),
            "(85) 99999-0000".into(),
            "maria@email.com".into(),
        )
        .unwrap();
        assert_eq!(paciente.nome.as_str(), "Maria");
        assert_eq!(paciente.cpf.as_str(), "12345678900");
    }

    #[test]
    fn novo_rejeita_campo_vazio() {
        let err = Paciente::novo(
            "".into(),
            "12345678900".into(),
            "1990-05-04".into(),
            "(85) 99999-0000".into(),
            "maria@email.com".into(),
        )
        .unwrap_err();
        assert_eq!(err, DomainError::CampoObrigatorio("nome"));
    }

    #[test]
    fn atualizar_altera_campos_permitidos() {
        let mut paciente = exemplo();
        paciente
            .atualizar("Maria Silva Souza".into(), "(85) 98888-1111".into(), "m.souza@email.com".into())
            .unwrap();
        assert_eq!(paciente.nome.as_str(), "Maria Silva Souza");
        assert_eq!(paciente.telefone.as_str(), "(85) 98888-1111");
        assert_eq!(paciente.email.as_str(), "m.souza@email.com");
    }

    #[test]
    fn atualizar_rejeita_nome_vazio() {
        let mut paciente = exemplo();
        let err = paciente
            .atualizar("  ".into(), "telefone".into(), "email".into())
            .unwrap_err();
        assert_eq!(err, DomainError::CampoObrigatorio("nome"));
    }
}