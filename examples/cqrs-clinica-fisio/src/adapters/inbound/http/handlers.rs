use axum::extract::{Json, Path, State};
use axum::http::StatusCode;

use crate::adapters::inbound::http::dto::{
    AtualizarPacienteRequest, AtualizarTerapiaRequest, CriarPacienteRequest, CriarTerapiaRequest,
    PacienteResponse, TerapiaResponse,
};
use crate::adapters::inbound::http::error::ApiError;
use crate::application::messages::paciente::{
    AtualizarPaciente, CriarPaciente, DeletarPaciente, ListarPacientes, ObterPaciente,
};
use crate::application::messages::terapia::{
    AtualizarTerapia, CriarTerapia, DeletarTerapia, ListarTerapias, ListarTerapiasDoPaciente,
    ObterTerapia,
};
use crate::bootstrap::AppState;

pub async fn listar_pacientes(
    State(state): State<AppState>,
) -> Result<Json<Vec<PacienteResponse>>, ApiError> {
    let pacientes = state.query_bus.execute(ListarPacientes).await?;
    Ok(Json(pacientes.into_iter().map(PacienteResponse::from).collect()))
}

pub async fn criar_paciente(
    State(state): State<AppState>,
    Json(input): Json<CriarPacienteRequest>,
) -> Result<(StatusCode, Json<PacienteResponse>), ApiError> {
    let paciente = state
        .command_bus
        .dispatch(CriarPaciente {
            nome: input.nome,
            cpf: input.cpf,
            data_nascimento: input.data_nascimento,
            telefone: input.telefone,
            email: input.email,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(PacienteResponse::from(paciente))))
}

pub async fn obter_paciente(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PacienteResponse>, ApiError> {
    let paciente = state
        .query_bus
        .execute(ObterPaciente { id })
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "paciente não encontrado"))?;
    Ok(Json(PacienteResponse::from(paciente)))
}

pub async fn atualizar_paciente(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<AtualizarPacienteRequest>,
) -> Result<Json<PacienteResponse>, ApiError> {
    let paciente = state
        .command_bus
        .dispatch(AtualizarPaciente {
            id,
            nome: input.nome,
            telefone: input.telefone,
            email: input.email,
        })
        .await?;
    Ok(Json(PacienteResponse::from(paciente)))
}

pub async fn deletar_paciente(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .command_bus
        .dispatch(DeletarPaciente { id })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn listar_terapias_do_paciente(
    State(state): State<AppState>,
    Path(paciente_id): Path<String>,
) -> Result<Json<Vec<TerapiaResponse>>, ApiError> {
    let terapias = state
        .query_bus
        .execute(ListarTerapiasDoPaciente { paciente_id })
        .await?;
    Ok(Json(terapias.into_iter().map(TerapiaResponse::from).collect()))
}

pub async fn listar_terapias(
    State(state): State<AppState>,
) -> Result<Json<Vec<TerapiaResponse>>, ApiError> {
    let terapias = state.query_bus.execute(ListarTerapias).await?;
    Ok(Json(terapias.into_iter().map(TerapiaResponse::from).collect()))
}

pub async fn criar_terapia(
    State(state): State<AppState>,
    Json(input): Json<CriarTerapiaRequest>,
) -> Result<(StatusCode, Json<TerapiaResponse>), ApiError> {
    let terapia = state
        .command_bus
        .dispatch(CriarTerapia {
            paciente_id: input.paciente_id,
            tipo: input.tipo,
            descricao: input.descricao,
            sessao_semanal: input.sessao_semanal,
            qtd_sessoes: input.qtd_sessoes,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(TerapiaResponse::from(terapia))))
}

pub async fn obter_terapia(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<TerapiaResponse>, ApiError> {
    let terapia = state
        .query_bus
        .execute(ObterTerapia { id })
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "terapia não encontrada"))?;
    Ok(Json(TerapiaResponse::from(terapia)))
}

pub async fn atualizar_terapia(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<AtualizarTerapiaRequest>,
) -> Result<Json<TerapiaResponse>, ApiError> {
    let terapia = state
        .command_bus
        .dispatch(AtualizarTerapia {
            id,
            descricao: input.descricao,
            sessao_semanal: input.sessao_semanal,
            qtd_sessoes: input.qtd_sessoes,
        })
        .await?;
    Ok(Json(TerapiaResponse::from(terapia)))
}

pub async fn deletar_terapia(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.command_bus.dispatch(DeletarTerapia { id }).await?;
    Ok(StatusCode::NO_CONTENT)
}