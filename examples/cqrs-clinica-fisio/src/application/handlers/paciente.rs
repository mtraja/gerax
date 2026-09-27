use std::sync::Arc;

use async_trait::async_trait;
use gerax_cqrs::{CommandHandler, CqrsError, QueryHandler};

use crate::application::errors::ApplicationError;
use crate::application::messages::paciente::{
    AtualizarPaciente, CriarPaciente, DeletarPaciente, ListarPacientes, ObterPaciente,
};
use crate::application::ports::outbound::PacienteRepository;
use crate::domain::Paciente;

pub struct CriarPacienteHandler<R> {
    repo: Arc<R>,
}

impl<R: PacienteRepository> CriarPacienteHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PacienteRepository + 'static> CommandHandler for CriarPacienteHandler<R> {
    type Command = CriarPaciente;

    async fn handle(&self, command: CriarPaciente) -> Result<Paciente, CqrsError> {
        let paciente = Paciente::novo(
            command.nome,
            command.cpf,
            command.data_nascimento,
            command.telefone,
            command.email,
        )
        .map_err(|e| ApplicationError::from(e).into_cqrs())?;
        self.repo.create(&paciente).await.map_err(|e| e.into_cqrs())
    }
}

pub struct AtualizarPacienteHandler<R> {
    repo: Arc<R>,
}

impl<R: PacienteRepository> AtualizarPacienteHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PacienteRepository + Send + Sync + 'static> CommandHandler for AtualizarPacienteHandler<R> {
    type Command = AtualizarPaciente;

    async fn handle(&self, command: AtualizarPaciente) -> Result<Paciente, CqrsError> {
        let mut paciente = self
            .repo
            .find(&command.id)
            .await
            .map_err(|e| e.into_cqrs())?
            .ok_or_else(|| {
                ApplicationError::not_found(format!("paciente {}", command.id)).into_cqrs()
            })?;
        paciente
            .atualizar(command.nome, command.telefone, command.email)
            .map_err(|e| ApplicationError::from(e).into_cqrs())?;
        self.repo.update(&paciente).await.map_err(|e| e.into_cqrs())
    }
}

pub struct DeletarPacienteHandler<R> {
    repo: Arc<R>,
}

impl<R: PacienteRepository> DeletarPacienteHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PacienteRepository + Send + Sync + 'static> CommandHandler for DeletarPacienteHandler<R> {
    type Command = DeletarPaciente;

    async fn handle(&self, command: DeletarPaciente) -> Result<(), CqrsError> {
        self.repo
            .delete(&command.id)
            .await
            .map_err(|e| e.into_cqrs())
    }
}

pub struct ObterPacienteHandler<R> {
    repo: Arc<R>,
}

impl<R: PacienteRepository> ObterPacienteHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PacienteRepository + Send + Sync + 'static> QueryHandler for ObterPacienteHandler<R> {
    type Query = ObterPaciente;

    async fn handle(&self, query: ObterPaciente) -> Result<Option<Paciente>, CqrsError> {
        self.repo.find(&query.id).await.map_err(|e| e.into_cqrs())
    }
}

pub struct ListarPacientesHandler<R> {
    repo: Arc<R>,
}

impl<R: PacienteRepository> ListarPacientesHandler<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PacienteRepository + Send + Sync + 'static> QueryHandler for ListarPacientesHandler<R> {
    type Query = ListarPacientes;

    async fn handle(&self, _query: ListarPacientes) -> Result<Vec<Paciente>, CqrsError> {
        self.repo.list().await.map_err(|e| e.into_cqrs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PacienteId;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakePacienteRepository {
        store: Mutex<HashMap<String, Paciente>>,
        next_id: Mutex<u64>,
    }

    impl FakePacienteRepository {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
                next_id: Mutex::new(1),
            }
        }
    }

    #[async_trait]
    impl PacienteRepository for FakePacienteRepository {
        async fn create(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let mut next_id = self.next_id.lock().unwrap();
            let mut salvo = paciente.clone();
            salvo.id = Some(PacienteId::new((*next_id).to_string()).unwrap());
            *next_id += 1;
            let id = salvo.id.clone().unwrap().to_string();
            store.insert(id, salvo.clone());
            Ok(salvo)
        }

        async fn update(&self, paciente: &Paciente) -> Result<Paciente, ApplicationError> {
            let mut store = self.store.lock().unwrap();
            let id = paciente
                .id
                .clone()
                .ok_or_else(|| ApplicationError::not_found("paciente sem id"))?
                .to_string();
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

    fn repo() -> Arc<FakePacienteRepository> {
        Arc::new(FakePacienteRepository::new())
    }

    fn exemplo() -> CriarPaciente {
        CriarPaciente {
            nome: "Maria Silva".into(),
            cpf: "12345678900".into(),
            data_nascimento: "1990-05-04".into(),
            telefone: "(85) 99999-0000".into(),
            email: "maria@email.com".into(),
        }
    }

    #[tokio::test]
    async fn criar_valida_campos_obrigatorios() {
        let handler = CriarPacienteHandler::new(repo());
        let mut command = exemplo();
        command.nome = "".into();
        let err = handler.handle(command).await.unwrap_err();
        assert!(matches!(err, CqrsError::Execution(msg) if msg.contains("campo obrigatório")));
    }

    #[tokio::test]
    async fn criar_retorna_paciente_com_id() {
        let handler = CriarPacienteHandler::new(repo());
        let paciente = handler.handle(exemplo()).await.unwrap();
        assert!(paciente.id.is_some());
        assert_eq!(paciente.nome.as_str(), "Maria Silva");
    }

    #[tokio::test]
    async fn atualizar_paciente_inexistente_retorna_not_found() {
        let handler = AtualizarPacienteHandler::new(repo());
        let err = handler
            .handle(AtualizarPaciente {
                id: "99".into(),
                nome: "Maria".into(),
                telefone: "(85) 90000-0000".into(),
                email: "m@email.com".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, CqrsError::Execution(msg) if msg.contains("[not_found]")));
    }

    #[tokio::test]
    async fn atualizar_retorna_entidade_atualizada() {
        let repo = repo();
        let criado = CriarPacienteHandler::new(repo.clone())
            .handle(exemplo())
            .await
            .unwrap();
        let atualizado = AtualizarPacienteHandler::new(repo.clone())
            .handle(AtualizarPaciente {
                id: criado.id.unwrap().to_string(),
                nome: "Maria Souza".into(),
                telefone: "(85) 98888-1111".into(),
                email: "m.souza@email.com".into(),
            })
            .await
            .unwrap();
        assert_eq!(atualizado.nome.as_str(), "Maria Souza");
    }

    #[tokio::test]
    async fn obter_inexistente_retorna_none() {
        let handler = ObterPacienteHandler::new(repo());
        let encontrado = handler
            .handle(ObterPaciente { id: "42".into() })
            .await
            .unwrap();
        assert!(encontrado.is_none());
    }

    #[tokio::test]
    async fn fluxo_completo_criar_listar_obter_deletar() {
        let repo = repo();
        let criado = CriarPacienteHandler::new(repo.clone())
            .handle(exemplo())
            .await
            .unwrap();

        let listados = ListarPacientesHandler::new(repo.clone())
            .handle(ListarPacientes)
            .await
            .unwrap();
        assert_eq!(listados.len(), 1);

        let encontrado = ObterPacienteHandler::new(repo.clone())
            .handle(ObterPaciente {
                id: criado.id.clone().unwrap().to_string(),
            })
            .await
            .unwrap();
        assert!(encontrado.is_some());

        DeletarPacienteHandler::new(repo.clone())
            .handle(DeletarPaciente {
                id: criado.id.unwrap().to_string(),
            })
            .await
            .unwrap();

        let vazio = ListarPacientesHandler::new(repo.clone())
            .handle(ListarPacientes)
            .await
            .unwrap();
        assert!(vazio.is_empty());
    }
}
