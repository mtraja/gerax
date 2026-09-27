use gerax_macros::{Command, Query};

use crate::domain::Terapia;

/// Cria um plano de terapia para um paciente.
#[derive(Debug, Command)]
#[command(output = Terapia)]
pub struct CriarTerapia {
    pub paciente_id: String,
    pub tipo: String,
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}

/// Atualiza uma terapia existente.
#[derive(Debug, Command)]
#[command(output = Terapia)]
pub struct AtualizarTerapia {
    pub id: String,
    pub descricao: String,
    pub sessao_semanal: u32,
    pub qtd_sessoes: u32,
}

/// Remove uma terapia.
#[derive(Debug, Command)]
#[command(output = ())]
pub struct DeletarTerapia {
    pub id: String,
}

/// Obtém uma terapia pelo id.
#[derive(Debug, Query)]
#[query(output = Option<Terapia>)]
pub struct ObterTerapia {
    pub id: String,
}

/// Lista todas as terapias.
#[derive(Debug, Query)]
#[query(output = Vec<Terapia>)]
pub struct ListarTerapias;

/// Lista as terapias de um paciente.
#[derive(Debug, Query)]
#[query(output = Vec<Terapia>)]
pub struct ListarTerapiasDoPaciente {
    pub paciente_id: String,
}
