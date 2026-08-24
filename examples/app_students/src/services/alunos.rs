use crate::models::{Aluno, AtualizarAluno, CriarAluno};
use crate::services::EntidadeCrud;
use gerax_app::Repository;

impl EntidadeCrud<CriarAluno, AtualizarAluno> for Aluno {
    fn novo(input: CriarAluno) -> Self {
        Self {
            id: None,
            nome: input.nome,
            email: input.email,
        }
    }

    fn aplicar_atualizacao(&mut self, input: AtualizarAluno) {
        self.nome = input.nome;
        self.email = input.email;
    }
}

servico_crud!(ServicoAluno, Aluno, CriarAluno, AtualizarAluno);
