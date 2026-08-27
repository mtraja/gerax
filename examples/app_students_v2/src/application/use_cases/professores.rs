use std::sync::Arc;

use crate::application::errors::ApplicationError;
use crate::application::ports::professor::ProfessorRepository;
use crate::domain::Professor;
#[allow(unused_imports)]
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct CriarProfessor {
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct AtualizarProfessor {
    pub id: String,
    pub nome: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct DeletarProfessor(pub String);

#[derive(Debug, Clone)]
pub struct ObterProfessor(pub String);

pub struct ProfessorUseCases<R> {
    repo: Arc<R>,
}

impl<R: ProfessorRepository> ProfessorUseCases<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn criar(&self, input: CriarProfessor) -> Result<Professor, ApplicationError> {
        let professor = Professor::novo(input.nome, input.email)?;
        self.repo.save(&professor).await
    }

    pub async fn atualizar(
        &self,
        input: AtualizarProfessor,
    ) -> Result<Professor, ApplicationError> {
        let mut professor = self
            .repo
            .find(&input.id)
            .await?
            .ok_or_else(|| ApplicationError::not_found(format!("professor {}", input.id)))?;
        professor.atualizar(input.nome, input.email)?;
        self.repo.save(&professor).await
    }

    pub async fn obter(
        &self,
        input: ObterProfessor,
    ) -> Result<Option<Professor>, ApplicationError> {
        self.repo.find(&input.0).await
    }

    pub async fn listar(&self) -> Result<Vec<Professor>, ApplicationError> {
        self.repo.list().await
    }

    pub async fn deletar(&self, input: DeletarProfessor) -> Result<(), ApplicationError> {
        self.repo.delete(&input.0).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeProfessorRepository {
        store: std::sync::Mutex<HashMap<String, Professor>>,
        next_id: std::sync::Mutex<u64>,
    }

    impl FakeProfessorRepository {
        fn new() -> Self {
            Self {
                store: std::sync::Mutex::new(HashMap::new()),
                next_id: std::sync::Mutex::new(1),
            }
        }
    }

    #[async_trait]
    impl ProfessorRepository for FakeProfessorRepository {
        async fn save(&self, professor: &Professor) -> Result<Professor, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let mut next_id = self.next_id.lock().unwrap();
            let id = professor
                .id
                .clone()
                .unwrap_or_else(|| (*next_id).to_string());
            if professor.id.is_none() {
                *next_id += 1;
            }
            let mut saved = professor.clone();
            saved.id = Some(id.clone());
            store.insert(id.clone(), saved.clone());
            Ok(saved)
        }

        async fn find(&self, id: &str) -> Result<Option<Professor>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store.get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<Professor>, ApplicationError> {
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
    async fn criar_professor_valida_campos() {
        let repo = Arc::new(FakeProfessorRepository::new());
        let use_case = ProfessorUseCases::new(repo);
        let result = use_case
            .criar(CriarProfessor {
                nome: "".into(),
                email: "p@b.com".into(),
            })
            .await;
        assert!(matches!(result, Err(ApplicationError { .. })));
    }
}
