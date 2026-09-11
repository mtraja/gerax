use thiserror::Error;

use crate::embedding::Embedding;
use crate::similarity::SimilarityError;

use super::filter::VectorFilter;

/// Erros possíveis nas operações do [`crate::vector::VectorStore`].
#[derive(Debug, Error)]
pub enum VectorStoreError {
    /// O documento vetorial não foi encontrado.
    #[error("document not found: {0}")]
    NotFound(String),
    /// A requisição de busca/operação é inválida.
    #[error("invalid search request: {0}")]
    InvalidRequest(String),
    /// Falha ao calcular a similaridade.
    #[error("similarity computation failed: {0}")]
    Similarity(#[from] SimilarityError),
    /// Falha interna do armazenamento.
    #[error("indexing state failed: {0}")]
    Internal(String),
}

/// Um documento persistido no armazenamento vetorial.
#[derive(Debug, Clone)]
pub struct VectorDocument {
    /// Identificador único do documento (tipicamente o `id` do chunk).
    pub id: String,
    /// Embedding associado ao conteúdo.
    pub vector: Embedding,
    /// Texto original do chunk.
    pub content: String,
    /// Metadados herdados do documento original, usados em filtros.
    pub metadata: crate::document::DocumentMetadata,
}

/// Resultado de uma busca vetorial: documento + pontuação de similaridade.
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    /// Documento correspondente ao resultado.
    pub document: VectorDocument,
    /// Pontuação de similaridade com o vetor da consulta.
    pub score: f32,
}

/// Requisição de busca vetorial por similaridade.
#[derive(Debug, Clone)]
pub struct VectorSearchRequest {
    /// Vetor da consulta.
    pub vector: Embedding,
    /// Quantidade máxima de resultados.
    pub limit: usize,
    /// Similaridade mínima para manter um resultado.
    pub min_score: Option<f32>,
    /// Filtro de metadados opcional aplicado à busca.
    pub filter: Option<VectorFilter>,
}
