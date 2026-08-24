use std::sync::Arc;

use gerax_app::{DbError, PostgresConnection, PostgresRepository, Repository};

use crate::models::{
    Aluno, AtualizarAluno, AtualizarProfessor, AtualizarTurma, CriarAluno, CriarMatricula,
    CriarProfessor, CriarTurma, Matricula, Professor, Turma,
};

pub trait EntidadeCrud<C, U> {
    fn novo(input: C) -> Self;
    fn aplicar_atualizacao(&mut self, input: U);
}

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

macro_rules! servico_crud {
    ($servico:ident, $entidade:ty, $criar:ty, $atualizar:ty) => {
        pub struct $servico {
            db: Arc<PostgresConnection>,
        }

        impl $servico {
            pub fn new(db: Arc<PostgresConnection>) -> Self {
                Self { db }
            }

            fn repo(&self) -> PostgresRepository<$entidade> {
                PostgresRepository::<$entidade>::new(self.db.clone())
            }

            pub async fn listar(&self) -> Result<Vec<$entidade>, DbError> {
                self.repo().find_all().await
            }

            pub async fn obter(&self, id: &str) -> Result<Option<$entidade>, DbError> {
                self.repo().find_by_id(id).await
            }

            pub async fn criar(&self, input: $criar) -> Result<$entidade, DbError> {
                let entidade = <$entidade>::novo(input);
                self.repo().insert(entidade).await
            }

            pub async fn atualizar(&self, id: &str, input: $atualizar) -> Result<$entidade, DbError> {
                let repo = self.repo();
                let mut entidade = repo
                    .find_by_id(id)
                    .await?
                    .ok_or_else(|| DbError::not_found(id))?;

                entidade.aplicar_atualizacao(input);
                repo.update(entidade).await?;

                repo.find_by_id(id)
                    .await?
                    .ok_or_else(|| DbError::not_found(id))
            }

            pub async fn deletar(&self, id: &str) -> Result<(), DbError> {
                self.repo().delete(id).await
            }
        }
    };
}

servico_crud!(ServicoAluno, Aluno, CriarAluno, AtualizarAluno);
servico_crud!(ServicoProfessor, Professor, CriarProfessor, AtualizarProfessor);
servico_crud!(ServicoTurma, Turma, CriarTurma, AtualizarTurma);

pub struct ServicoMatricula {
    db: Arc<PostgresConnection>,
}

impl ServicoMatricula {
    pub fn new(db: Arc<PostgresConnection>) -> Self {
        Self { db }
    }

    fn repo(&self) -> PostgresRepository<Matricula> {
        PostgresRepository::<Matricula>::new(self.db.clone())
    }

    pub async fn listar(&self) -> Result<Vec<Matricula>, DbError> {
        self.repo().find_all().await
    }

    pub async fn obter(&self, id: &str) -> Result<Option<Matricula>, DbError> {
        self.repo().find_by_id(id).await
    }

    pub async fn criar(&self, input: CriarMatricula) -> Result<Matricula, DbError> {
        let matricula = Matricula {
            id: None,
            aluno_id: input.aluno_id,
            turma_id: input.turma_id,
        };
        self.repo().insert(matricula).await
    }

    pub async fn deletar(&self, id: &str) -> Result<(), DbError> {
        self.repo().delete(id).await
    }
}
