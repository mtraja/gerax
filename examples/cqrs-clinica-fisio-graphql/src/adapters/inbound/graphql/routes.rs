//! Rotas HTTP do adaptador GraphQL (axum).
//!
//! Expõe `POST /graphql` para execução de operações e `GET /graphiql`
//! para a interface gráfica de desenvolvimento.

use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use gerax_graphql::{GraphiQL, GraphqlRequest, GraphqlResponse};
use tower_http::cors::CorsLayer;

use crate::bootstrap::AppState;

/// Processa uma requisição `POST /graphql`.
async fn graphql_handler(
    State(state): State<AppState>,
    Json(request): Json<GraphqlRequest>,
) -> Response {
    let response = match state
        .graphql_schema
        .execute_with_state(&request.query, request.variables, Arc::new(state.clone()))
        .await
    {
        Ok(executed) => executed.into_response(),
        Err(err) => GraphqlResponse {
            data: None,
            errors: Some(vec![err]),
            extensions: None,
        },
    };
    Json(response).into_response()
}

/// Interface gráfica (GraphiQL) em `GET /graphiql`.
async fn graphiql() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        GraphiQL::new("/graphql").render(),
    )
}

/// Router do adaptador GraphQL.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/graphiql", get(graphiql))
        .layer(CorsLayer::permissive())
}
