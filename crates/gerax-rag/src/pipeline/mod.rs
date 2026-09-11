//! Orquestração ponta a ponta do fluxo RAG (recuperação → reranking → contexto).

mod pipeline_impl;
mod types;

pub use pipeline_impl::{RagError, RagPipeline};
pub use types::*;
