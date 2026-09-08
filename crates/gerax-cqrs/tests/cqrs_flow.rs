//! Testes de integração do gerax-cqrs com as macros derivadas.

use std::sync::Arc;

use async_trait::async_trait;

use gerax_cqrs::{
    Command, CommandBus, CommandHandler, CommandMetadata, CqrsError, HandlerRegistry, Message,
    Query, QueryBus, QueryHandler, QueryMetadata, register_commands, register_queries,
};
use gerax_macros::{Command, Query};

#[derive(Debug, Clone, PartialEq)]
struct Aluno {
    id: u64,
    nome: String,
}

#[derive(Command)]
#[command(output = Aluno)]
struct CreateAluno {
    nome: String,
}

#[derive(Command)]
#[command(output = Matricula)]
struct MatricularAluno {
    aluno_id: u64,
}

#[derive(Command)]
#[command(output = Matricula)]
struct CancelarMatricula {
    matricula_id: u64,
}

#[derive(Command)]
#[command(output = Vec<Aluno>)]
struct ListarAlunos;

#[derive(Debug, Clone, PartialEq)]
struct Matricula {
    id: u64,
    aluno_id: u64,
    ativa: bool,
}

#[derive(Query)]
#[query(output = Option<Aluno>)]
struct GetAluno {
    id: u64,
}

#[derive(Query)]
#[query(output = Option<Vec<Aluno>>)]
struct ListAlunos;

struct CreateAlunoHandler {
    next_id: u64,
}

#[async_trait]
impl CommandHandler for CreateAlunoHandler {
    type Command = CreateAluno;

    async fn handle(&self, command: CreateAluno) -> Result<Aluno, CqrsError> {
        Ok(Aluno {
            id: self.next_id,
            nome: command.nome,
        })
    }
}

struct MatricularAlunoHandler;

#[async_trait]
impl CommandHandler for MatricularAlunoHandler {
    type Command = MatricularAluno;

    async fn handle(&self, command: MatricularAluno) -> Result<Matricula, CqrsError> {
        Ok(Matricula {
            id: 1,
            aluno_id: command.aluno_id,
            ativa: true,
        })
    }
}

struct CancelarMatriculaHandler;

#[async_trait]
impl CommandHandler for CancelarMatriculaHandler {
    type Command = CancelarMatricula;

    async fn handle(&self, command: CancelarMatricula) -> Result<Matricula, CqrsError> {
        Ok(Matricula {
            id: command.matricula_id,
            aluno_id: 0,
            ativa: false,
        })
    }
}

struct ListarAlunosHandler;

#[async_trait]
impl CommandHandler for ListarAlunosHandler {
    type Command = ListarAlunos;

    async fn handle(&self, _command: ListarAlunos) -> Result<Vec<Aluno>, CqrsError> {
        Ok(vec![Aluno {
            id: 1,
            nome: "Marcos".into(),
        }])
    }
}

struct GetAlunoHandler;

#[async_trait]
impl QueryHandler for GetAlunoHandler {
    type Query = GetAluno;

    async fn handle(&self, query: GetAluno) -> Result<Option<Aluno>, CqrsError> {
        if query.id == 1 {
            Ok(Some(Aluno {
                id: 1,
                nome: "Marcos".into(),
            }))
        } else {
            Ok(None)
        }
    }
}

struct ListAlunosHandlerQuery;

#[async_trait]
impl QueryHandler for ListAlunosHandlerQuery {
    type Query = ListAlunos;

    async fn handle(&self, _query: ListAlunos) -> Result<Option<Vec<Aluno>>, CqrsError> {
        Ok(Some(vec![Aluno {
            id: 1,
            nome: "Marcos".into(),
        }]))
    }
}

fn build_registry() -> Result<HandlerRegistry, CqrsError> {
    let mut registry = HandlerRegistry::new();
    register_commands!(
        registry,
        CreateAlunoHandler { next_id: 10 },
        MatricularAlunoHandler,
        CancelarMatriculaHandler,
        ListarAlunosHandler,
    );
    register_queries!(registry, GetAlunoHandler, ListAlunosHandlerQuery,);
    Ok(registry)
}

// Teste 1 — Command
#[tokio::test]
async fn teste_1_command_fluxo_completo() {
    let registry = Arc::new(build_registry().unwrap());
    let bus = CommandBus::new(registry.clone());

    let aluno = bus
        .dispatch(CreateAluno {
            nome: "Marcos".into(),
        })
        .await
        .unwrap();

    assert_eq!(
        aluno,
        Aluno {
            id: 10,
            nome: "Marcos".into()
        }
    );
}

// Teste 2 — Query
#[tokio::test]
async fn teste_2_query_fluxo_completo() {
    let registry = Arc::new(build_registry().unwrap());
    let bus = QueryBus::new(registry);

    let aluno = bus.execute(GetAluno { id: 1 }).await.unwrap();
    assert_eq!(aluno.unwrap().nome, "Marcos");

    let vazio = bus.execute(GetAluno { id: 99 }).await.unwrap();
    assert!(vazio.is_none());
}

// Teste 3 — Handler inexistente
#[tokio::test]
async fn teste_3_handler_inexistente() {
    #[derive(Command)]
    #[command(output = Aluno)]
    struct CommandSemHandler;

    let registry = Arc::new(HandlerRegistry::new());
    let bus = CommandBus::new(registry.clone());

    let err = bus.dispatch(CommandSemHandler).await.unwrap_err();
    assert!(matches!(err, CqrsError::HandlerNotFound { .. }));
}

// Teste 4 — Registro duplicado
#[test]
fn teste_4_registro_duplicado() {
    let mut registry = HandlerRegistry::new();
    registry
        .register_command(CreateAlunoHandler { next_id: 1 })
        .unwrap();

    let err = registry
        .register_command(CreateAlunoHandler { next_id: 2 })
        .unwrap_err();

    assert!(matches!(err, CqrsError::HandlerAlreadyRegistered));
}

// Teste 5 — Commands diferentes
#[tokio::test]
async fn teste_5_commands_diferentes() {
    let registry = Arc::new(build_registry().unwrap());
    let bus = CommandBus::new(registry.clone());

    let aluno = bus
        .dispatch(CreateAluno { nome: "Ana".into() })
        .await
        .unwrap();
    assert_eq!(aluno.id, 10);

    let matricula = bus.dispatch(MatricularAluno { aluno_id: 5 }).await.unwrap();
    assert!(matricula.ativa);

    let cancelada = bus
        .dispatch(CancelarMatricula { matricula_id: 9 })
        .await
        .unwrap();
    assert!(!cancelada.ativa);

    let lista = bus.dispatch(ListarAlunos).await.unwrap();
    assert_eq!(lista.len(), 1);
}

// Teste 6 — Queries diferentes
#[tokio::test]
async fn teste_6_queries_diferentes() {
    let registry = Arc::new(build_registry().unwrap());
    let bus = QueryBus::new(registry);

    let aluno = bus.execute(GetAluno { id: 1 }).await.unwrap();
    assert_eq!(aluno.unwrap().nome, "Marcos");

    let lista = bus.execute(ListAlunos).await.unwrap();
    assert_eq!(lista.unwrap().len(), 1);
}

// Teste 7 — Metadata
#[test]
fn teste_7_metadata() {
    assert_eq!(CreateAluno::NAME, "CreateAluno");
    assert_eq!(GetAluno::NAME, "GetAluno");
    assert_eq!(ListarAlunos::NAME, "ListarAlunos");
    assert_eq!(MatricularAluno::NAME, "MatricularAluno");
    assert_eq!(CancelarMatricula::NAME, "CancelarMatricula");
    assert_eq!(ListAlunos::NAME, "ListAlunos");
}

// Testes de macro — tipos complexos de output
#[derive(Command)]
#[command(output = Vec<Aluno>)]
struct CommandVecAluno;

#[derive(Query)]
#[query(output = Option<Vec<Aluno>>)]
struct QueryOptionVecAluno;

#[derive(Query)]
#[query(output = Result<Aluno, CqrsError>)]
struct QueryResultAluno;

#[test]
fn macros_com_tipos_complexos_compilam() {
    // Esses tipos apenas precisam compilar; os names são verificados abaixo.
    assert_eq!(CommandVecAluno::NAME, "CommandVecAluno");
    assert_eq!(QueryOptionVecAluno::NAME, "QueryOptionVecAluno");
    assert_eq!(QueryResultAluno::NAME, "QueryResultAluno");
}

#[test]
fn message_output_types_are_derived() {
    fn assert_output<T: Message + Command, R: Send + Sync + 'static>() {}
    fn assert_query_output<Q: Message + Query, R: Send + Sync + 'static>() {}
    let _ = assert_output::<CreateAluno, Aluno>;
    let _ = assert_query_output::<GetAluno, Option<Aluno>>;
}

// Criação de registro e buses no padrão da API pública (§26)
#[tokio::test]
async fn api_padrao_registro_manuais() {
    let mut registry = HandlerRegistry::new();
    registry
        .register_command(CreateAlunoHandler { next_id: 1 })
        .unwrap();
    registry.register_query(GetAlunoHandler).unwrap();

    let registry = Arc::new(registry);

    let command_bus = CommandBus::new(registry.clone());
    let query_bus = QueryBus::new(registry);

    let aluno = command_bus
        .dispatch(CreateAluno {
            nome: "Camila".into(),
        })
        .await
        .unwrap();
    assert_eq!(aluno.id, 1);

    let consulta = query_bus.execute(GetAluno { id: 1 }).await.unwrap();
    assert_eq!(consulta.unwrap().nome, "Marcos");
}
