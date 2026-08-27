use std::sync::Arc;

use crate::application::errors::ApplicationError;
use crate::application::ports::aluno::AlunoRepository;
use crate::domain::Aluno;
#[allow(unused_imports)]
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CriarAluno {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct AtualizarAluno {
    pub id: String,
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct DeletarAluno(pub String);

#[derive(Debug, Clone)]
pub struct ObterAluno(pub String);

pub struct AlunoUseCases<R> {
    repo: Arc<R>,
}

impl<R: AlunoRepository> AlunoUseCases<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn criar(&self, input: CriarAluno) -> Result<Aluno, ApplicationError> {
        let aluno = Aluno::novo(input.nome, input.email)?;
        self.repo.save(&aluno).await
    }

    pub async fn atualizar(&self, input: AtualizarAluno) -> Result<Aluno, ApplicationError> {
        let mut aluno = self
            .repo
            .find(&input.id)
            .await?
            .ok_or_else(|| ApplicationError::not_found(format!("aluno {}", input.id)))?;
        aluno.atualizar(input.nome, input.email)?;
        self.repo.save(&aluno).await
    }

    pub async fn obter(&self, input: ObterAluno) -> Result<Option<Aluno>, ApplicationError> {
        self.repo.find(&input.0).await
    }

    pub async fn listar(&self) -> Result<Vec<Aluno>, ApplicationError> {
        self.repo.list().await
    }

    pub async fn deletar(&self, input: DeletarAluno) -> Result<(), ApplicationError> {
        self.repo.delete(&input.0).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    struct FakeAlunoRepository {
        store: std::sync::Mutex<HashMap<String, Aluno>>,
        next_id: std::sync::Mutex<u64>,
    }

    impl FakeAlunoRepository {
        fn new() -> Self {
            Self {
                store: std::sync::Mutex::new(HashMap::new()),
                next_id: std::sync::Mutex::new(1),
            }
        }
    }

    #[async_trait]
    impl AlunoRepository for FakeAlunoRepository {
        async fn save(&self, aluno: &Aluno) -> Result<Aluno, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let mut next_id = self.next_id.lock().unwrap();
            let id = aluno.id.clone().unwrap_or_else(|| (*next_id).to_string());
            if aluno.id.is_none() {
                *next_id += 1;
            }
            let mut saved = aluno.clone();
            saved.id = Some(id.clone());
            store.insert(id.clone(), saved.clone());
            Ok(saved)
        }

        async fn find(&self, id: &str) -> Result<Option<Aluno>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store.get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<Aluno>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store.values().cloned().collect())
        }

        async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
            let mut store = self.store.lock().unwrap();
            store.remove(id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn criar_aluno_valida_campos() {
        let repo = Arc::new(FakeAlunoRepository::new());
        let use_case = AlunoUseCases::new(repo);
        let result = use_case
            .criar(CriarAluno {
                nome: "".into(),
                email: "a@b.com".into(),
            })
            .await;
        assert!(matches!(result, Err(ApplicationError { .. })));
    }

    #[tokio::test]
    async fn criar_aluno_salva_e_retorna_entidade() {
        let repo = Arc::new(FakeAlunoRepository::new());
        let use_case = AlunoUseCases::new(repo);
        let result = use_case
            .criar(CriarAluno {
                nome: "Joao".into(),
                email: "joao@b.com".into(),
            })
            .await;
        assert!(result.is_ok());
        let aluno = result.unwrap();
        assert!(aluno.id.is_some());
        assert_eq!(aluno.nome, "Joao");
    }

    #[tokio::test]
    async fn atualizar_aluno_nao_encontrado() {
        let repo = Arc::new(FakeAlunoRepository::new());
        let use_case = AlunoUseCases::new(repo);
        let result = use_case
            .atualizar(AtualizarAluno {
                id: "99".into(),
                nome: "Joao".into(),
                email: "joao@b.com".into(),
            })
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("aluno 99"));
    }

    #[tokio::test]
    async fn atualizar_aluno_existente() {
        let repo = Arc::new(FakeAlunoRepository::new());
        let use_case = AlunoUseCases::new(repo);
        let criado = use_case
            .criar(CriarAluno {
                nome: "Joao".into(),
                email: "joao@b.com".into(),
            })
            .await
            .unwrap();
        let result = use_case
            .atualizar(AtualizarAluno {
                id: criado.id.clone().unwrap(),
                nome: "Joao Silva".into(),
                email: "joao.silva@b.com".into(),
            })
            .await;
        assert!(result.is_ok());
        let atualizado = result.unwrap();
        assert_eq!(atualizado.nome, "Joao Silva");
    }

    #[tokio::test]
    async fn deletar_aluno_remove_entidade() {
        let repo = Arc::new(FakeAlunoRepository::new());
        let use_case = AlunoUseCases::new(repo);
        let criado = use_case
            .criar(CriarAluno {
                nome: "Joao".into(),
                email: "joao@b.com".into(),
            })
            .await
            .unwrap();
        let result = use_case
            .deletar(DeletarAluno(criado.id.clone().unwrap()))
            .await;
        assert!(result.is_ok());
        let found = use_case
            .obter(ObterAluno(criado.id.unwrap()))
            .await
            .unwrap();
        assert!(found.is_none());
    }
}
