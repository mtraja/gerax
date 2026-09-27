use serde::{Deserialize, Serialize};

use crate::domain::{Paciente, Terapia};

#[derive(Debug, Serialize)]
pub struct PacienteResponse {
    pub id: String,
    pub nome: String,
    pub cpf: String,
    pub data_nascimento: String,
    pub telefone: String,
    pub email: String,
}

impl From<Paciente> for PacienteResponse {
    fn from(value: Paciente) -> Self {
        Self {
            id: value.id.map(|id| id.to_string()).unwrap_or_default(),
            nome: value.nome.into(),
            cpf: value.cpf.into(),
            data_nascimento: value.data_nascimento.into(),
            telefone: value.telefone.into(),
            email: value.email.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CriarPacienteRequest {
    pub nome: String,
    pub cpf: String,
    pub data_nascimento: String,
    pub telefone: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct AtualizarPacienteRequest {
    pub nome: String,
    pub telefone: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct TerapiaResponse {
    pub id: String,
    pub paciente_id: String,
    pub tipo: String,
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}

impl From<Terapia> for TerapiaResponse {
    fn from(value: Terapia) -> Self {
        Self {
            id: value.id.map(|id| id.to_string()).unwrap_or_default(),
            paciente_id: value.paciente_id.into(),
            tipo: value.tipo.into(),
            descricao: value.descricao.into(),
            sessao_semanal: value.sessao_semanal.into(),
            qtd_sessoes: value.qtd_sessoes.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CriarTerapiaRequest {
    pub paciente_id: String,
    pub tipo: String,
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}

#[derive(Debug, Deserialize)]
pub struct AtualizarTerapiaRequest {
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}
