pub trait EntidadeCrud<C, U> {
    fn novo(input: C) -> Self;
    fn aplicar_atualizacao(&mut self, input: U);
}

macro_rules! servico_crud {
    ($servico:ident, $entidade:ty, $criar:ty, $atualizar:ty) => {
        pub struct $servico {
            db: ::std::sync::Arc<::gerax_app::PostgresConnection>,
        }

        impl $servico {
            pub fn new(db: ::std::sync::Arc<::gerax_app::PostgresConnection>) -> Self {
                Self { db }
            }

            fn repo(&self) -> ::gerax_app::PostgresRepository<$entidade> {
                ::gerax_app::PostgresRepository::<$entidade>::new(self.db.clone())
            }

            pub async fn listar(&self) -> Result<Vec<$entidade>, ::gerax_app::DbError> {
                self.repo().find_all().await
            }

            pub async fn obter(&self, id: &str) -> Result<Option<$entidade>, ::gerax_app::DbError> {
                self.repo().find_by_id(id).await
            }

            pub async fn criar(&self, input: $criar) -> Result<$entidade, ::gerax_app::DbError> {
                let entidade = <$entidade>::novo(input);
                self.repo().insert(entidade).await
            }

            pub async fn atualizar(
                &self,
                id: &str,
                input: $atualizar,
            ) -> Result<$entidade, ::gerax_app::DbError> {
                let repo = self.repo();
                let mut entidade = repo
                    .find_by_id(id)
                    .await?
                    .ok_or_else(|| ::gerax_app::DbError::not_found(id))?;

                entidade.aplicar_atualizacao(input);
                repo.update(entidade).await?;

                repo.find_by_id(id)
                    .await?
                    .ok_or_else(|| ::gerax_app::DbError::not_found(id))
            }

            pub async fn deletar(&self, id: &str) -> Result<(), ::gerax_app::DbError> {
                self.repo().delete(id).await
            }
        }
    };
}

mod alunos;
mod professores;
mod turmas;
mod matriculas;

pub use alunos::ServicoAluno;
pub use professores::ServicoProfessor;
pub use turmas::ServicoTurma;
pub use matriculas::ServicoMatricula;
