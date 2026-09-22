//! Adaptador de entrada via API REST (axum).

mod dto;
mod error;
mod handlers;
mod routes;

pub use routes::router;