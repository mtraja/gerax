//! Armazenamento vetorial, busca por similaridade e filtros de metadados.

mod filter;
mod in_memory;
mod store;
mod types;

pub use filter::{Filter, VectorFilter};
pub use in_memory::InMemoryVectorStore;
pub use store::VectorStore;
pub use types::*;
