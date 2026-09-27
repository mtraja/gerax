use serde::{Deserialize, Serialize};

use super::error::DomainError;
use super::value_objects::{
    Descricao, PacienteId, QtdSessoes, SessoesSemanais, TerapiaId, TipoTerapia,
};

/// Entidade `Terapia`: plano de fisioterapia vinculado a um `Paciente`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Terapia {
    pub id: Option<TerapiaId>,
    pub paciente_id: PacienteId,
    pub tipo: TipoTerapia,
    pub descricao: Descricao,
    pub sessao_semanal: SessoesSemanais,
    pub qtd_sessoes: QtdSessoes,
}

impl Terapia {
    pub fn nova(
        paciente_id: String,
        tipo: String,
        descricao: String,
        sessao_semanal: u32,
        qtd_sessoes: u32,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: None,
            paciente_id: PacienteId::new(paciente_id)?,
            tipo: TipoTerapia::new(tipo)?,
            descricao: Descricao::new(descricao)?,
            sessao_semanal: SessoesSemanais::new(sessao_semanal)?,
            qtd_sessoes: QtdSessoes::new(qtd_sessoes)?,
        })
    }

    pub fn atualizar(
        &mut self,
        descricao: String,
        sessao_semanal: u32,
        qtd_sessoes: u32,
    ) -> Result<(), DomainError> {
        self.descricao = Descricao::new(descricao)?;
        self.sessao_semanal = SessoesSemanais::new(sessao_semanal)?;
        self.qtd_sessoes = QtdSessoes::new(qtd_sessoes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemplo() -> Terapia {
        Terapia::nova(
            "p1".into(),
            "Fisioterapia Ortopédica".into(),
            "Reabilitação de joelho".into(),
            3,
            10,
        )
        .unwrap()
    }

    #[test]
    fn nova_cria_terapia_sem_id() {
        let terapia = exemplo();
        assert_eq!(terapia.id, None);
        assert_eq!(terapia.paciente_id.as_str(), "p1");
        assert_eq!(terapia.sessao_semanal.value(), 3);
        assert_eq!(terapia.qtd_sessoes.value(), 10);
    }

    #[test]
    fn nova_rejeita_tipo_vazio() {
        let err = Terapia::nova("p1".into(), "  ".into(), "descrição".into(), 2, 8).unwrap_err();
        assert_eq!(err, DomainError::CampoObrigatorio("tipo"));
    }

    #[test]
    fn atualizar_altera_descricao_e_volumes() {
        let mut terapia = exemplo();
        terapia
            .atualizar("Reabilitação pós-operatória".into(), 4, 12)
            .unwrap();
        assert_eq!(terapia.descricao.as_str(), "Reabilitação pós-operatória");
        assert_eq!(terapia.sessao_semanal.value(), 4);
        assert_eq!(terapia.qtd_sessoes.value(), 12);
    }
}
