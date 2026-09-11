//! Divisão de documentos em trechos (chunks) antes da indexação.
//!
//! Define o trait [`Chunker`] e as estratégias de chunking disponíveis:
//! [`FixedSizeChunker`], [`RecursiveChunker`] e [`MarkdownChunker`].

mod algorithm;
mod fixed_size;
mod markdown;
mod recursive;
mod types;

pub use fixed_size::{FixedSizeChunker, FixedSizeChunkerError};
pub use markdown::{MarkdownChunker, MarkdownChunkerError};
pub use recursive::{RecursiveChunker, RecursiveChunkerError};
pub use types::*;
