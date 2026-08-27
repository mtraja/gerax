use std::sync::Arc;

use gerax_app::{App, Connection, PostgresConnection};
use gerax_http::CorsConfig;

use crate::adapters::inbound::http::routes;
use crate::adapters::outbound::postgres::{
    PostgresAlunoPorTurmaQuery, PostgresAlunoRepository, PostgresMatriculaRepository,
    PostgresProfessorRepository, PostgresTurmaRepository,
};
use crate::application::use_cases::alunos::AlunoUseCases;
use crate::application::use_cases::matriculas::MatriculaUseCases;
use crate::application::use_cases::professores::ProfessorUseCases;
use crate::application::use_cases::turmas::TurmaUseCases;

pub struct AppState {
    pub alunos: Arc<AlunoUseCases<PostgresAlunoRepository>>,
    pub professores: Arc<ProfessorUseCases<PostgresProfessorRepository>>,
    pub turmas: Arc<TurmaUseCases<PostgresTurmaRepository>>,
    pub matriculas: Arc<
        MatriculaUseCases<
            PostgresMatriculaRepository,
            PostgresAlunoRepository,
            PostgresTurmaRepository,
            PostgresAlunoPorTurmaQuery,
        >,
    >,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("gerax_postgres=debug,app_students_v2=info")
            }),
        )
        .try_init();

    let connection = Arc::new(PostgresConnection::connect().await?);
    connection.ping().await?;

    PostgresAlunoRepository::new(connection.clone())
        .create_table()
        .await?;
    PostgresProfessorRepository::new(connection.clone())
        .create_table()
        .await?;
    PostgresTurmaRepository::new(connection.clone())
        .create_table()
        .await?;
    PostgresMatriculaRepository::new(connection.clone())
        .create_table()
        .await?;

    let aluno_repo = Arc::new(PostgresAlunoRepository::new(connection.clone()));
    let professor_repo = Arc::new(PostgresProfessorRepository::new(connection.clone()));
    let turma_repo = Arc::new(PostgresTurmaRepository::new(connection.clone()));
    let matricula_repo = Arc::new(PostgresMatriculaRepository::new(connection.clone()));
    let aluno_por_turma_query = Arc::new(PostgresAlunoPorTurmaQuery::new(connection.clone()));

    let alunos = Arc::new(AlunoUseCases::new(aluno_repo.clone()));
    let professores = Arc::new(ProfessorUseCases::new(professor_repo));
    let turmas = Arc::new(TurmaUseCases::new(turma_repo.clone()));
    let matriculas = Arc::new(MatriculaUseCases::new(
        matricula_repo,
        aluno_repo,
        turma_repo,
        aluno_por_turma_query,
    ));

    let state = AppState {
        alunos,
        professores,
        turmas,
        matriculas,
    };

    let router = routes::routes();

    let cors = CorsConfig::new().allowed_origin("http://localhost:5173");

    App::builder(state)
        .router(router)
        .cors(cors)
        .build()
        .run::<gerax_app::ActixRuntime>()
        .await?;

    Ok(())
}
