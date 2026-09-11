use thiserror::Error;

/// Erros possíveis ao criar uma [`Query`].
#[derive(Debug, Error)]
pub enum QueryError {
    /// A consulta está vazia ou contém apenas espaços em branco.
    #[error("query cannot be empty")]
    EmptyQuery,
}

/// Consulta de busca validada como texto não vazio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    text: String,
}

impl Query {
    /// Cria uma consulta a partir de um texto, rejeitando textos em branco.
    pub fn new(text: impl Into<String>) -> Result<Self, QueryError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(QueryError::EmptyQuery);
        }
        Ok(Self { text })
    }

    /// Retorna o texto da consulta.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl From<Query> for String {
    fn from(query: Query) -> Self {
        query.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_query() {
        let query = Query::new("como cancelar matricula").unwrap();
        assert_eq!(query.text(), "como cancelar matricula");
    }

    #[test]
    fn rejects_empty_query() {
        assert!(matches!(Query::new(""), Err(QueryError::EmptyQuery)));
        assert!(matches!(Query::new("   "), Err(QueryError::EmptyQuery)));
    }
}
