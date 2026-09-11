//! Reordenação dos resultados recuperados antes da montagem do contexto.

mod noop;
mod types;

pub use noop::NoopReranker;
pub use types::*;
