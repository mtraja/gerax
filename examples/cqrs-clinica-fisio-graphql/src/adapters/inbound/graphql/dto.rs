//! Tipos GraphQL de entrada (`InputObject`) e saída (`SimpleObject`).
//!
//! O adaptador expõe o domínio em camelCase, convenção do GraphQL,
//! mantendo as entidades `Paciente` e `Terapia` isoladas.

use async_graphql::{InputObject, SimpleObject};

use crate::domain::{Paciente, Terapia};

#[derive(Debug, Clone, SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct PacienteType {
    pub id: String,
    pub nome: String,
    pub cpf: String,
    pub data_nascimento: String,
    pub telefone: String,
    pub email: String,
}

impl From<Paciente> for PacienteType {
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

#[derive(Debug, Clone, SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct TerapiaType {
    pub id: String,
    pub paciente_id: String,
    pub tipo: String,
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}

impl From<Terapia> for TerapiaType {
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

#[derive(Debug, Clone, InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct CriarPacienteInput {
    pub nome: String,
    pub cpf: String,
    pub data_nascimento: String,
    pub telefone: String,
    pub email: String,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct AtualizarPacienteInput {
    pub nome: String,
    pub telefone: String,
    pub email: String,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct CriarTerapiaInput {
    pub paciente_id: String,
    pub tipo: String,
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}

#[derive(Debug, Clone, InputObject)]
#[graphql(rename_fields = "camelCase")]
pub struct AtualizarTerapiaInput {
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}
