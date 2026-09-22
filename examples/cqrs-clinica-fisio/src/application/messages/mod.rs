//! Mensagens CQRS (Commands e Queries).
//!
//! As mensagens definem a intenção (`Command`) ou a consulta (`Query`)
//! e o tipo de resultado via `#[command(output = ...)]` /
//! `#[query(output = ...)]`. Elas não conhecem seus handlers.

pub mod paciente;
pub mod terapia;