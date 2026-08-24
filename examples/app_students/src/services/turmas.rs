use crate::models::{AtualizarTurma, CriarTurma, Turma};
use crate::services::EntidadeCrud;
use gerax_app::Repository;

impl EntidadeCrud<CriarTurma, AtualizarTurma> for Turma {
    fn novo(input: CriarTurma) -> Self {
        Self {
            id: None,
            nome: input.nome,
            professor_id: input.professor_id,
        }
    }

    fn aplicar_atualizacao(&mut self, input: AtualizarTurma) {
        self.nome = input.nome;
        self.professor_id = input.professor_id;
    }
}

servico_crud!(ServicoTurma, Turma, CriarTurma, AtualizarTurma);
