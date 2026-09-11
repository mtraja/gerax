use crate::vector::{VectorFilter, VectorSearchResult};

/// Requisição de execução do pipeline RAG.
#[derive(Debug, Clone)]
pub struct RagRequest {
    /// Texto da consulta do usuário.
    pub query: String,
    /// Quantidade máxima de documentos recuperados no estágio de retrieval.
    pub retrieval_limit: usize,
    /// Pontuação mínima para manter um documento recuperado.
    pub min_score: Option<f32>,
    /// Filtro de metadados opcional aplicado na recuperação.
    pub filter: Option<VectorFilter>,
}

impl RagRequest {
    /// Cria uma requisição com consulta e limite de recuperação.
    pub fn new(query: impl Into<String>, retrieval_limit: usize) -> Self {
        Self {
            query: query.into(),
            retrieval_limit,
            min_score: None,
            filter: None,
        }
    }

    /// Define a similaridade mínima dos resultados, encadeável.
    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = Some(min_score);
        self
    }

    /// Define um filtro de metadados, encadeável.
    pub fn with_filter(mut self, filter: VectorFilter) -> Self {
        self.filter = Some(filter);
        self
    }
}

/// Resultado da execução do pipeline RAG.
#[derive(Debug)]
pub struct RagResult {
    /// Texto da consulta executada.
    pub query: String,
    /// Documentos recuperados (e reordenados pelo reranker, se configurado).
    pub documents: Vec<VectorSearchResult>,
    /// Contexto textual montado para alimentar o prompt do LLM.
    pub context: String,
}
