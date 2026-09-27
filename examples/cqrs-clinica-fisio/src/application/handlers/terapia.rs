use std::sync::Arc;

use async_trait::async_trait;
use gerax_cqrs::{CommandHandler, CqrsError, QueryHandler};

use crate::application::errors::ApplicationError;
use crate::application::messages::terapia::{
    AtualizarTerapia, CriarTerapia, DeletarTerapia, ListarTerapias, ListarTerapiasDoPaciente,
    ObterTerapia,
};
use crate::application::ports::outbound::{PacienteRepository, TerapiaRepository};
use crate::domain::Terapia;

pub struct CriarTerapiaHandler<R, P> {
    terapia_repo: Arc<R>,
    paciente_repo: Arc<P>,
}

impl<R: TerapiaRepository, P: PacienteRepository> CriarTerapiaHandler<R, P> {
    pub fn new(terapia_repo: Arc<R>, paciente_repo: Arc<P>) -> Self {
        Self {
            terapia_repo,
            paciente_repo,
        }
    }
}

#[async_trait]
impl<R: TerapiaRepository + 'static, P: PacienteRepository + 'static> CommandHandler
    for CriarTerapiaHandler<R, P>
{
    type Command = CriarTerapia;

    async fn handle(&self, command: CriarTerapia) -> Result<Terapia, CqrsError> {
        let paciente = self
            .paciente_repo
            .find(&command.paciente_id)
            .await
            .map_err(|e| e.into_cqrs())?;
        if paciente.is_none() {
            return Err(ApplicationError::business_rule(format!(
                "paciente {} não existe",
                command.paciente_id
            ))
            .into_cqrs());
        }

        let terapia = Terapia::nova(
            command.paciente_id,
            command.tipo,
            command.descricao,
            command.sessao_semanal,
            command.qtd_sessoes,
        )
        .map_err(|e| ApplicationError::from(e).into_cqrs())?;
        self.terapia_repo
            .create(&terapia)
            .await
            .map_err(|e| e.into_cqrs())
    }
}

pub struct AtualizarTerapiaHandler<R> {
    repo: Arc<R>,
}

impl<R: TerapiaRepository> AtualizarTerapiaHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: TerapiaRepository + Send + Sync + 'static> CommandHandler for AtualizarTerapiaHandler<R> {
    type Command = AtualizarTerapia;

    async fn handle(&self, command: AtualizarTerapia) -> Result<Terapia, CqrsError> {
        let mut terapia = self
            .repo
            .find(&command.id)
            .await
            .map_err(|e| e.into_cqrs())?
            .ok_or_else(|| {
                ApplicationError::not_found(format!("terapia {}", command.id)).into_cqrs()
            })?;
        terapia
            .atualizar(
                command.descricao,
                command.sessao_semanal,
                command.qtd_sessoes,
            )
            .map_err(|e| ApplicationError::from(e).into_cqrs())?;
        self.repo.update(&terapia).await.map_err(|e| e.into_cqrs())
    }
}

pub struct DeletarTerapiaHandler<R> {
    repo: Arc<R>,
}

impl<R: TerapiaRepository> DeletarTerapiaHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: TerapiaRepository + Send + Sync + 'static> CommandHandler for DeletarTerapiaHandler<R> {
    type Command = DeletarTerapia;

    async fn handle(&self, command: DeletarTerapia) -> Result<(), CqrsError> {
        self.repo
            .delete(&command.id)
            .await
            .map_err(|e| e.into_cqrs())
    }
}

pub struct ObterTerapiaHandler<R> {
    repo: Arc<R>,
}

impl<R: TerapiaRepository> ObterTerapiaHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: TerapiaRepository + Send + Sync + 'static> QueryHandler for ObterTerapiaHandler<R> {
    type Query = ObterTerapia;

    async fn handle(&self, query: ObterTerapia) -> Result<Option<Terapia>, CqrsError> {
        self.repo.find(&query.id).await.map_err(|e| e.into_cqrs())
    }
}

pub struct ListarTerapiasHandler<R> {
    repo: Arc<R>,
}

impl<R: TerapiaRepository> ListarTerapiasHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: TerapiaRepository + Send + Sync + 'static> QueryHandler for ListarTerapiasHandler<R> {
    type Query = ListarTerapias;

    async fn handle(&self, _query: ListarTerapias) -> Result<Vec<Terapia>, CqrsError> {
        self.repo.list().await.map_err(|e| e.into_cqrs())
    }
}

pub struct ListarTerapiasDoPacienteHandler<R> {
    repo: Arc<R>,
}

impl<R: TerapiaRepository> ListarTerapiasDoPacienteHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: TerapiaRepository + Send + Sync + 'static> QueryHandler
    for ListarTerapiasDoPacienteHandler<R>
{
    type Query = ListarTerapiasDoPaciente;

    async fn handle(&self, query: ListarTerapiasDoPaciente) -> Result<Vec<Terapia>, CqrsError> {
        self.repo
            .list_by_paciente(&query.paciente_id)
            .await
            .map_err(|e| e.into_cqrs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::outbound::PacienteRepository;
    use crate::domain::{Paciente, PacienteId, TerapiaId};
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakePacienteRepository {
        store: Mutex<HashMap<String, Paciente>>,
    }

    impl FakePacienteRepository {
        fn with(id: &str) -> Self {
            let mut store = HashMap::new();
            let mut paciente = Paciente::novo(
                "Maria Silva".into(),
                "12345678900".into(),
                "1990-05-04".into(),
                "(85) 99999-0000".into(),
                "maria@email.com".into(),
            )
            .unwrap();
            paciente.id = Some(PacienteId::new(id.to_string()).unwrap());
            store.insert(id.to_string(), paciente);
            Self {
                store: Mutex::new(store),
            }
        }
    }

    #[async_trait]
    impl PacienteRepository for FakePacienteRepository {
        async fn create(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let id = paciente
                .id
                .clone()
                .map(|id| id.to_string())
                .unwrap_or_default();
            store.insert(id, paciente.clone());
            Ok(paciente.clone())
        }

        async fn update(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let id = paciente
                .id
                .clone()
                .map(|id| id.to_string())
                .unwrap_or_default();
            store.insert(id, paciente.clone());
            Ok(paciente.clone())
        }

        async fn find(&self, id: &str) -> Result<Option<Paciente>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store.get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<Paciente>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store.values().cloned().collect())
        }

        async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
            let mut store = self.store.lock().unwrap();
            store.remove(id);
            Ok(())
        }
    }

    struct FakeTerapiaRepository {
        store: Mutex<HashMap<String, Terapia>>,
        next_id: Mutex<u64>,
    }

    impl FakeTerapiaRepository {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
                next_id: Mutex::new(1),
            }
        }
    }

    #[async_trait]
    impl TerapiaRepository for FakeTerapiaRepository {
        async fn create(&self, terapia: &Terapia) -> Result<Terapia, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let mut next_id = self.next_id.lock().unwrap();
            let mut salvo = terapia.clone();
            salvo.id = Some(TerapiaId::new((*next_id).to_string()).unwrap());
            *next_id += 1;
            let id = salvo.id.clone().unwrap().to_string();
            store.insert(id, salvo.clone());
            Ok(salvo)
        }

        async fn update(&self, terapia: &Terapia) -> Result<Terapia, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let id = terapia
                .id
                .clone()
                .ok_or_else(|| ApplicationError::not_found("terapia sem id"))?
                .to_string();
            store.insert(id, terapia.clone());
            Ok(terapia.clone())
        }

        async fn find(&self, id: &str) -> Result<Option<Terapia>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store.get(id).cloned())
        }

        async fn list(&self) -> Result<Vec<Terapia>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store.values().cloned().collect())
        }

        async fn list_by_paciente(
            &self,
            paciente_id: &str,
        ) -> Result<Vec<Terapia>, ApplicationError> {
            let store = self.store.lock().unwrap();
            Ok(store
                .values()
                .filter(|t| t.paciente_id.as_str() == paciente_id)
                .cloned()
                .collect())
        }

        async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
            let mut store = self.store.lock().unwrap();
            store.remove(id);
            Ok(())
        }
    }

    fn exemplo(paciente_id: &str) -> CriarTerapia {
        CriarTerapia {
            paciente_id: paciente_id.to_string(),
            tipo: "Fisioterapia Ortopédica".into(),
            descricao: "Reabilitação de joelho".into(),
            sessao_semanal: 3,
            qtd_sessoes: 10,
        }
    }

    #[tokio::test]
    async fn criar_rejeita_paciente_inexistente() {
        let terapia_repo = Arc::new(FakeTerapiaRepository::new());
        let paciente_repo = Arc::new(FakePacienteRepository::with("p1"));
        let handler = CriarTerapiaHandler::new(terapia_repo, paciente_repo);

        let err = handler
            .handle(exemplo("paciente-que-nao-existe"))
            .await
            .unwrap_err();
        assert!(matches!(err, CqrsError::Execution(msg) if msg.contains("[business_rule]")));
    }

    #[tokio::test]
    async fn criar_valeida_paciente_e_salva_terapia() {
        let terapia_repo = Arc::new(FakeTerapiaRepository::new());
        let paciente_repo = Arc::new(FakePacienteRepository::with("p1"));
        let handler = CriarTerapiaHandler::new(terapia_repo.clone(), paciente_repo);

        let terapia = handler.handle(exemplo("p1")).await.unwrap();
        assert!(terapia.id.is_some());
        assert_eq!(terapia.paciente_id.as_str(), "p1");
    }

    #[tokio::test]
    async fn atualizar_terapia_inexistente_retorna_not_found() {
        let repo = Arc::new(FakeTerapiaRepository::new());
        let handler = AtualizarTerapiaHandler::new(repo);
        let err = handler
            .handle(AtualizarTerapia {
                id: "42".into(),
                descricao: "nova descrição".into(),
                sessao_semanal: 2,
                qtd_sessoes: 8,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CqrsError::Execution(msg) if msg.contains("[not_found]")));
    }

    #[tokio::test]
    async fn listar_por_paciente_filtra_corretamente() {
        let terapia_repo = Arc::new(FakeTerapiaRepository::new());
        let paciente_repo = Arc::new(FakePacienteRepository::with("p1"));
        let criar = CriarTerapiaHandler::new(terapia_repo.clone(), paciente_repo.clone());
        criar.handle(exemplo("p1")).await.unwrap();
        criar.handle(exemplo("p1")).await.unwrap();

        let listar = ListarTerapiasDoPacienteHandler::new(terapia_repo.clone());
        let da_p1 = listar
            .handle(ListarTerapiasDoPaciente {
                paciente_id: "p1".into(),
            })
            .await
            .unwrap();
        assert_eq!(da_p1.len(), 2);

        let da_outra = listar
            .handle(ListarTerapiasDoPaciente {
                paciente_id: "p2".into(),
            })
            .await
            .unwrap();
        assert!(da_outra.is_empty());
    }
}
