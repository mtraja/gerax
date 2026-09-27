use axum::Router;
use axum::routing::get;

use crate::adapters::inbound::http::handlers::{
    atualizar_paciente, atualizar_terapia, criar_paciente, criar_terapia, deletar_paciente,
    deletar_terapia, listar_pacientes, listar_terapias, listar_terapias_do_paciente,
    obter_paciente, obter_terapia,
};
use crate::bootstrap::AppState;

/// Rotas da API da clínica de fisioterapia.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pacientes", get(listar_pacientes).post(criar_paciente))
        .route(
            "/pacientes/{id}",
            get(obter_paciente)
                .put(atualizar_paciente)
                .delete(deletar_paciente),
        )
        .route("/pacientes/{id}/terapias", get(listar_terapias_do_paciente))
        .route("/terapias", get(listar_terapias).post(criar_terapia))
        .route(
            "/terapias/{id}",
            get(obter_terapia)
                .put(atualizar_terapia)
                .delete(deletar_terapia),
        )
}
