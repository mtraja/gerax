use gerax_app::Context;
use gerax_http::{ServerResult, routing::Response};
use serde::Serialize;

use crate::adapters::inbound::http::dto::{
    AlunoResponse, AtualizarAlunoRequest, AtualizarProfessorRequest, AtualizarTurmaRequest,
    CriarAlunoRequest, CriarMatriculaRequest, CriarProfessorRequest, CriarTurmaRequest,
    MatriculaResponse, ProfessorResponse, TurmaResponse,
};
use crate::application::errors::ApplicationError;
use crate::application::ports::inbound::alunos::{AtualizarAluno, CriarAluno, DeletarAluno, ObterAluno};
use crate::application::ports::inbound::matriculas::{CriarMatricula, DeletarMatricula, ObterMatricula};
use crate::application::ports::inbound::professores::{
    AtualizarProfessor, CriarProfessor, DeletarProfessor, ObterProfessor,
};
use crate::application::ports::inbound::turmas::{AtualizarTurma, CriarTurma, DeletarTurma, ObterTurma};
use crate::bootstrap::AppState;

pub async fn listar_alunos(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().alunos.listar().await.map_err(http_err)?;
    json_response(
        &items
            .into_iter()
            .map(AlunoResponse::from)
            .collect::<Vec<_>>(),
    )
}

pub async fn obter_aluno(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    match ctx
        .state()
        .alunos
        .obter(ObterAluno(id.clone()))
        .await
        .map_err(http_err)?
    {
        Some(item) => json_response(&AlunoResponse::from(item)),
        None => Ok(Response::not_found()),
    }
}

pub async fn criar_aluno(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CriarAlunoRequest = serde_json::from_slice(ctx.request().body())
        .map_err(|e| http_err(ApplicationError::infrastructure(e.to_string())))?;
    let item = ctx
        .state()
        .alunos
        .criar(CriarAluno {
            nome: input.nome,
            email: input.email,
        })
        .await
        .map_err(http_err)?;
    json_response(&AlunoResponse::from(item))
}

pub async fn atualizar_aluno(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    let input: AtualizarAlunoRequest = serde_json::from_slice(ctx.request().body())
        .map_err(|e| http_err(ApplicationError::infrastructure(e.to_string())))?;
    let item = ctx
        .state()
        .alunos
        .atualizar(AtualizarAluno {
            id,
            nome: input.nome,
            email: input.email,
        })
        .await
        .map_err(http_err)?;
    json_response(&AlunoResponse::from(item))
}

pub async fn deletar_aluno(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    ctx.state()
        .alunos
        .deletar(DeletarAluno(id))
        .await
        .map_err(http_err)?;
    Ok(Response::ok(b"{}".to_vec()))
}

pub async fn listar_professores(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().professores.listar().await.map_err(http_err)?;
    json_response(
        &items
            .into_iter()
            .map(ProfessorResponse::from)
            .collect::<Vec<_>>(),
    )
}

pub async fn obter_professor(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    match ctx
        .state()
        .professores
        .obter(ObterProfessor(id.clone()))
        .await
        .map_err(http_err)?
    {
        Some(item) => json_response(&ProfessorResponse::from(item)),
        None => Ok(Response::not_found()),
    }
}

pub async fn criar_professor(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CriarProfessorRequest = serde_json::from_slice(ctx.request().body())
        .map_err(|e| http_err(ApplicationError::infrastructure(e.to_string())))?;
    let item = ctx
        .state()
        .professores
        .criar(CriarProfessor {
            nome: input.nome,
            email: input.email,
        })
        .await
        .map_err(http_err)?;
    json_response(&ProfessorResponse::from(item))
}

pub async fn atualizar_professor(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    let input: AtualizarProfessorRequest = serde_json::from_slice(ctx.request().body())
        .map_err(|e| http_err(ApplicationError::infrastructure(e.to_string())))?;
    let item = ctx
        .state()
        .professores
        .atualizar(AtualizarProfessor {
            id,
            nome: input.nome,
            email: input.email,
        })
        .await
        .map_err(http_err)?;
    json_response(&ProfessorResponse::from(item))
}

pub async fn deletar_professor(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    ctx.state()
        .professores
        .deletar(DeletarProfessor(id))
        .await
        .map_err(http_err)?;
    Ok(Response::ok(b"{}".to_vec()))
}

pub async fn listar_turmas(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().turmas.listar().await.map_err(http_err)?;
    json_response(
        &items
            .into_iter()
            .map(TurmaResponse::from)
            .collect::<Vec<_>>(),
    )
}

pub async fn obter_turma(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    match ctx
        .state()
        .turmas
        .obter(ObterTurma(id.clone()))
        .await
        .map_err(http_err)?
    {
        Some(item) => json_response(&TurmaResponse::from(item)),
        None => Ok(Response::not_found()),
    }
}

pub async fn criar_turma(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CriarTurmaRequest = serde_json::from_slice(ctx.request().body())
        .map_err(|e| http_err(ApplicationError::infrastructure(e.to_string())))?;
    let item = ctx
        .state()
        .turmas
        .criar(CriarTurma {
            nome: input.nome,
            professor_id: input.professor_id,
        })
        .await
        .map_err(http_err)?;
    json_response(&TurmaResponse::from(item))
}

pub async fn atualizar_turma(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    let input: AtualizarTurmaRequest = serde_json::from_slice(ctx.request().body())
        .map_err(|e| http_err(ApplicationError::infrastructure(e.to_string())))?;
    let item = ctx
        .state()
        .turmas
        .atualizar(AtualizarTurma {
            id,
            nome: input.nome,
            professor_id: input.professor_id,
        })
        .await
        .map_err(http_err)?;
    json_response(&TurmaResponse::from(item))
}

pub async fn deletar_turma(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    ctx.state()
        .turmas
        .deletar(DeletarTurma(id))
        .await
        .map_err(http_err)?;
    Ok(Response::ok(b"{}".to_vec()))
}

pub async fn listar_alunos_por_turma(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    let items = ctx
        .state()
        .matriculas
        .listar_alunos_por_turma(&id)
        .await
        .map_err(http_err)?;
    json_response(
        &items
            .into_iter()
            .map(AlunoResponse::from)
            .collect::<Vec<_>>(),
    )
}

pub async fn listar_matriculas(ctx: Context<AppState>) -> ServerResult<Response> {
    let items = ctx.state().matriculas.listar().await.map_err(http_err)?;
    json_response(
        &items
            .into_iter()
            .map(MatriculaResponse::from)
            .collect::<Vec<_>>(),
    )
}

pub async fn obter_matricula(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    match ctx
        .state()
        .matriculas
        .obter(ObterMatricula(id.clone()))
        .await
        .map_err(http_err)?
    {
        Some(item) => json_response(&MatriculaResponse::from(item)),
        None => Ok(Response::not_found()),
    }
}

pub async fn criar_matricula(ctx: Context<AppState>) -> ServerResult<Response> {
    let input: CriarMatriculaRequest = serde_json::from_slice(ctx.request().body())
        .map_err(|e| http_err(ApplicationError::infrastructure(e.to_string())))?;
    let item = ctx
        .state()
        .matriculas
        .criar(CriarMatricula {
            aluno_id: input.aluno_id,
            turma_id: input.turma_id,
        })
        .await
        .map_err(http_err)?;
    json_response(&MatriculaResponse::from(item))
}

pub async fn deletar_matricula(ctx: Context<AppState>) -> ServerResult<Response> {
    let id = path_id(&ctx)?;
    ctx.state()
        .matriculas
        .deletar(DeletarMatricula(id))
        .await
        .map_err(http_err)?;
    Ok(Response::ok(b"{}".to_vec()))
}

fn path_id<S>(ctx: &Context<S>) -> Result<String, gerax_http::HttpServerError> {
    ctx.params()
        .get("id")
        .map(str::to_owned)
        .ok_or_else(|| gerax_http::HttpServerError::HandlerError("identificador ausente".into()))
}

fn http_err(err: ApplicationError) -> gerax_http::HttpServerError {
    gerax_http::HttpServerError::HandlerError(err.to_string())
}

fn json_response<T: Serialize>(value: &T) -> ServerResult<Response> {
    let body = serde_json::to_vec(value)
        .map_err(|e| gerax_http::HttpServerError::HandlerError(e.to_string()))?;
    Ok(Response::ok(body))
}
