use crate::models::{AtualizarProfessor, CriarProfessor, Professor};
use crate::services::EntidadeCrud;
use gerax_app::Repository;

impl EntidadeCrud<CriarProfessor, AtualizarProfessor> for Professor {
    fn novo(input: CriarProfessor) -> Self {
        Self {
            id: None,
            nome: input.nome,
            email: input.email,
        }
    }

    fn aplicar_atualizacao(&mut self, input: AtualizarProfessor) {
        self.nome = input.nome;
        self.email = input.email;
    }
}

servico_crud!(ServicoProfessor, Professor, CriarProfessor, AtualizarProfessor);
