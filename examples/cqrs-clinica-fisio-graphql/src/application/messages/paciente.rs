use gerax_macros::{Command, Query};

use crate::domain::Paciente;

/// Cria um novo paciente.
#[derive(Debug, Command)]
#[command(output = Paciente)]
pub struct CriarPaciente {
    pub nome: String,
    pub cpf: String,
    pub data_nascimento: String,
    pub telefone: String,
    pub email: String,
}

/// Atualiza um paciente existente.
#[derive(Debug, Command)]
#[command(output = Paciente)]
pub struct AtualizarPaciente {
    pub id: String,
    pub nome: String,
    pub telefone: String,
    pub email: String,
}

/// Remove um paciente.
#[derive(Debug, Command)]
#[command(output = ())]
pub struct DeletarPaciente {
    pub id: String,
}

/// Obtém um paciente pelo id.
#[derive(Debug, Query)]
#[query(output = Option<Paciente>)]
pub struct ObterPaciente {
    pub id: String,
}

/// Lista todos os pacientes.
#[derive(Debug, Query)]
#[query(output = Vec<Paciente>)]
pub struct ListarPacientes;