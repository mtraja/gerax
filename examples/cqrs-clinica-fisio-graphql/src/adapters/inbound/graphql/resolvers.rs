//! Raízes GraphQL de query e mutation.
//!
//! Cada resolver despacha comandos/consultas pelas buses de `AppState`,
//! traduzindo o domínio para os tipos GraphQL do adaptador.

use async_graphql::{Context, Object};

use crate::adapters::inbound::graphql::dto::{
    AtualizarPacienteInput, AtualizarTerapiaInput, CriarPacienteInput, CriarTerapiaInput,
    PacienteType, TerapiaType,
};
use crate::adapters::inbound::graphql::error::map_cqrs;
use crate::application::messages::paciente::{
    AtualizarPaciente, CriarPaciente, DeletarPaciente, ListarPacientes, ObterPaciente,
};
use crate::application::messages::terapia::{
    AtualizarTerapia, CriarTerapia, DeletarTerapia, ListarTerapias, ListarTerapiasDoPaciente,
    ObterTerapia,
};
use crate::bootstrap::AppState;

#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    #[graphql(name = "pacientes")]
    async fn listar_pacientes(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::FieldResult<Vec<PacienteType>> {
        let state = ctx.data::<AppState>()?;
        let pacientes = state
            .query_bus
            .execute(ListarPacientes)
            .await
            .map_err(map_cqrs)?;
        Ok(pacientes.into_iter().map(PacienteType::from).collect())
    }

    #[graphql(name = "paciente")]
    async fn obter_paciente(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::FieldResult<Option<PacienteType>> {
        let state = ctx.data::<AppState>()?;
        let paciente = state
            .query_bus
            .execute(ObterPaciente { id })
            .await
            .map_err(map_cqrs)?;
        Ok(paciente.map(PacienteType::from))
    }

    #[graphql(name = "terapias")]
    async fn listar_terapias(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::FieldResult<Vec<TerapiaType>> {
        let state = ctx.data::<AppState>()?;
        let terapias = state
            .query_bus
            .execute(ListarTerapias)
            .await
            .map_err(map_cqrs)?;
        Ok(terapias.into_iter().map(TerapiaType::from).collect())
    }

    #[graphql(name = "terapia")]
    async fn obter_terapia(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::FieldResult<Option<TerapiaType>> {
        let state = ctx.data::<AppState>()?;
        let terapia = state
            .query_bus
            .execute(ObterTerapia { id })
            .await
            .map_err(map_cqrs)?;
        Ok(terapia.map(TerapiaType::from))
    }

    #[graphql(name = "terapiasDoPaciente")]
    async fn listar_terapias_do_paciente(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "pacienteId")] paciente_id: String,
    ) -> async_graphql::FieldResult<Vec<TerapiaType>> {
        let state = ctx.data::<AppState>()?;
        let terapias = state
            .query_bus
            .execute(ListarTerapiasDoPaciente { paciente_id })
            .await
            .map_err(map_cqrs)?;
        Ok(terapias.into_iter().map(TerapiaType::from).collect())
    }
}

#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    #[graphql(name = "criarPaciente")]
    async fn criar_paciente(
        &self,
        ctx: &Context<'_>,
        input: CriarPacienteInput,
    ) -> async_graphql::FieldResult<PacienteType> {
        let state = ctx.data::<AppState>()?;
        let paciente = state
            .command_bus
            .dispatch(CriarPaciente {
                nome: input.nome,
                cpf: input.cpf,
                data_nascimento: input.data_nascimento,
                telefone: input.telefone,
                email: input.email,
            })
            .await
            .map_err(map_cqrs)?;
        Ok(PacienteType::from(paciente))
    }

    #[graphql(name = "atualizarPaciente")]
    async fn atualizar_paciente(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: AtualizarPacienteInput,
    ) -> async_graphql::FieldResult<PacienteType> {
        let state = ctx.data::<AppState>()?;
        let paciente = state
            .command_bus
            .dispatch(AtualizarPaciente {
                id,
                nome: input.nome,
                telefone: input.telefone,
                email: input.email,
            })
            .await
            .map_err(map_cqrs)?;
        Ok(PacienteType::from(paciente))
    }

    #[graphql(name = "deletarPaciente")]
    async fn deletar_paciente(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::FieldResult<bool> {
        let state = ctx.data::<AppState>()?;
        state
            .command_bus
            .dispatch(DeletarPaciente { id })
            .await
            .map_err(map_cqrs)?;
        Ok(true)
    }

    #[graphql(name = "criarTerapia")]
    async fn criar_terapia(
        &self,
        ctx: &Context<'_>,
        input: CriarTerapiaInput,
    ) -> async_graphql::FieldResult<TerapiaType> {
        let state = ctx.data::<AppState>()?;
        let terapia = state
            .command_bus
            .dispatch(CriarTerapia {
                paciente_id: input.paciente_id,
                tipo: input.tipo,
                descricao: input.descricao,
                sessao_semanal: input.sessao_semanal,
                qtd_sessoes: input.qtd_sessoes,
            })
            .await
            .map_err(map_cqrs)?;
        Ok(TerapiaType::from(terapia))
    }

    #[graphql(name = "atualizarTerapia")]
    async fn atualizar_terapia(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: AtualizarTerapiaInput,
    ) -> async_graphql::FieldResult<TerapiaType> {
        let state = ctx.data::<AppState>()?;
        let terapia = state
            .command_bus
            .dispatch(AtualizarTerapia {
                id,
                descricao: input.descricao,
                sessao_semanal: input.sessao_semanal,
                qtd_sessoes: input.qtd_sessoes,
            })
            .await
            .map_err(map_cqrs)?;
        Ok(TerapiaType::from(terapia))
    }

    #[graphql(name = "deletarTerapia")]
    async fn deletar_terapia(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::FieldResult<bool> {
        let state = ctx.data::<AppState>()?;
        state
            .command_bus
            .dispatch(DeletarTerapia { id })
            .await
            .map_err(map_cqrs)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use gerax_cqrs::{
        CommandBus, HandlerRegistry, QueryBus, register_commands, register_queries,
    };
    use serde_json::json;

    use crate::adapters::inbound::graphql::schema;
    use crate::application::errors::ApplicationError;
    use crate::application::handlers::paciente::{
        AtualizarPacienteHandler, CriarPacienteHandler, DeletarPacienteHandler,
        ListarPacientesHandler, ObterPacienteHandler,
    };
    use crate::application::handlers::terapia::{
        AtualizarTerapiaHandler, CriarTerapiaHandler, DeletarTerapiaHandler,
        ListarTerapiasDoPacienteHandler, ListarTerapiasHandler, ObterTerapiaHandler,
    };
    use crate::application::ports::outbound::{PacienteRepository, TerapiaRepository};
    use crate::bootstrap::AppState;
    use crate::domain::{Paciente, PacienteId, Terapia, TerapiaId};

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

    fn estado() -> Result<(AppState, Arc<FakePacienteRepository>, Arc<FakeTerapiaRepository>), gerax_cqrs::CqrsError> {
        let paciente_repo = Arc::new(FakePacienteRepository::new());
        let terapia_repo = Arc::new(FakeTerapiaRepository::new());

        let mut registry = HandlerRegistry::new();
        register_commands!(
            registry,
            CriarPacienteHandler::new(paciente_repo.clone()),
            AtualizarPacienteHandler::new(paciente_repo.clone()),
            DeletarPacienteHandler::new(paciente_repo.clone()),
            CriarTerapiaHandler::new(terapia_repo.clone(), paciente_repo.clone()),
            AtualizarTerapiaHandler::new(terapia_repo.clone()),
            DeletarTerapiaHandler::new(terapia_repo.clone()),
        );
        register_queries!(
            registry,
            ObterPacienteHandler::new(paciente_repo.clone()),
            ListarPacientesHandler::new(paciente_repo.clone()),
            ObterTerapiaHandler::new(terapia_repo.clone()),
            ListarTerapiasHandler::new(terapia_repo.clone()),
            ListarTerapiasDoPacienteHandler::new(terapia_repo.clone()),
        );

        let registry = Arc::new(registry);
        let state = AppState {
            command_bus: CommandBus::new(registry.clone()),
            query_bus: QueryBus::new(registry),
            graphql_schema: Arc::new(schema()),
        };
        Ok((state, paciente_repo, terapia_repo))
    }

    async fn executar(state: &AppState, query: &str) -> gerax_graphql::GraphqlResponse {
        state
            .graphql_schema
            .execute_with_state(query, None, Arc::new(state.clone()))
            .await
            .unwrap()
            .into_response()
    }

    #[tokio::test]
    async fn mutation_cria_e_query_lista_pacientes() {
        let (state, _, _) = estado().unwrap();

        let resposta = executar(
            &state,
            r#"mutation {
                criarPaciente(input: {
                    nome: "Maria Silva"
                    cpf: "12345678900"
                    dataNascimento: "1990-05-04"
                    telefone: "(85) 99999-0000"
                    email: "maria@email.com"
                }) { id nome cpf }
            }"#,
        )
        .await;

        let dados = resposta.clone().data.unwrap();
        let nome = dados["criarPaciente"]["nome"].as_str().unwrap();
        let id = dados["criarPaciente"]["id"].as_str().unwrap().to_string();
        assert_eq!(nome, "Maria Silva");

        let listados = executar(&state, "{ pacientes { id nome } }").await;
        assert_eq!(
            listados.data,
            Some(json!({ "pacientes": [{ "id": id, "nome": "Maria Silva" }] }))
        );
    }

    #[tokio::test]
    async fn query_obtem_paciente_por_id() {
        let (state, repo, _) = estado().unwrap();
        let mut paciente =
            Paciente::novo("João".into(), "00000000000".into(), "1980-01-01".into(), "85".into(), "joao@email.com".into())
                .unwrap();
        paciente.id = Some(PacienteId::new("p1".to_string()).unwrap());
        let salvo = repo.create(&paciente).await.unwrap();
        let id = salvo.id.clone().unwrap().to_string();

        let resposta = executar(
            &state,
            &format!("{{ paciente(id: \"{id}\") {{ id nome email }} }}"),
        )
        .await;
        assert_eq!(
            resposta.data,
            Some(json!({ "paciente": { "id": id, "nome": "João", "email": "joao@email.com" } }))
        );
    }

    #[tokio::test]
    async fn criacao_de_terapia_viola_regra_de_negocio() {
        let (state, _, _) = estado().unwrap();

        let resposta = executar(
            &state,
            r#"mutation {
                criarTerapia(input: {
                    pacienteId: "inexistente"
                    tipo: "Fisioterapia Ortopédica"
                    descricao: "Reabilitação de joelho"
                    sessaoSemanal: 3
                    qtdSessoes: 10
                }) { id }
            }"#,
        )
        .await;

        let response = resposta;
        assert!(response.data.is_none());
        let erros = response.errors.expect("deveria ter erro");
        assert_eq!(erros.len(), 1);
        assert!(erros[0].to_string().contains("não existe"));
    }

    #[tokio::test]
    async fn fluxo_completo_paciente_com_terapias() {
        let (state, _, _) = estado().unwrap();
        let criada = executar(
            &state,
            r#"mutation {
                criarPaciente(input: {
                    nome: "Ana" cpf: "11111111111" dataNascimento: "2000-01-01"
                    telefone: "85" email: "ana@email.com"
                }) { id }
            }"#,
        )
        .await;
        let paciente_id = criada.data.unwrap()["criarPaciente"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let _ = executar(
            &state,
            &format!(
                r#"mutation {{
                    criarTerapia(input: {{
                        pacienteId: "{paciente_id}"
                        tipo: "Fisioterapia Esportiva"
                        descricao: "Recuperação de tendão"
                        sessaoSemanal: 2
                        qtdSessoes: 6
                    }}) {{ id }}
                }}"#
            ),
        )
        .await;

        let resposta = executar(&state, "{ terapiasDoPaciente(pacienteId: \"1\") { id tipo } }").await;
        assert_eq!(
            resposta.data,
            Some(json!({ "terapiasDoPaciente": [{ "id": "1", "tipo": "Fisioterapia Esportiva" }] }))
        );
    }
}