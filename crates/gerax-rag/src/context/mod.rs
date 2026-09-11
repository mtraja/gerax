//! Montagem do contexto textual enviado ao LLM a partir dos resultados.

mod builder;
mod types;

pub use builder::{ContextConfig, SimpleContextBuilder};
pub use types::*;
