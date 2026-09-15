//! Módulo de Resources.
//!
//! Resources são fontes de dados legíveis expostas pelo servidor MCP
//! para fornecer contexto a modelos de linguagem. Cada resource é
//! identificado unicamente por uma URI.

mod registry;
mod trait_def;

pub use registry::{FailingResource, ResourceRegistry, StaticResource};
pub use trait_def::{
    BlobResourceContents, Resource, ResourceContents, ResourceError, TextResourceContents,
};
