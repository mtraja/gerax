use crate::document::DocumentMetadata;

/// Predicado atômico de comparação sobre um campo de metadados.
#[derive(Debug, Clone)]
pub enum Filter {
    /// Igualdade exata: `metadata[key] == value`.
    Eq {
        key: String,
        value: serde_json::Value,
    },
    /// Pertinência a uma lista: `metadata[key] ∈ values`.
    In {
        key: String,
        values: Vec<serde_json::Value>,
    },
}

impl Filter {
    /// Constrói um filtro de igualdade em `key` com o valor `value`.
    pub fn eq(key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Filter::Eq {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Constrói um filtro de pertinência em `key` com a lista `values`.
    pub fn in_list(key: impl Into<String>, values: Vec<serde_json::Value>) -> Self {
        Filter::In {
            key: key.into(),
            values,
        }
    }
}

/// Conjunto de filtros combinados por AND sobre metadados de documentos.
#[derive(Debug, Clone, Default)]
pub struct VectorFilter {
    and: Vec<Filter>,
}

impl VectorFilter {
    /// Cria um filtro combinando todos os predicados com AND.
    pub fn and(filters: impl IntoIterator<Item = Filter>) -> Self {
        Self {
            and: filters.into_iter().collect(),
        }
    }

    /// Verifica se `metadata` satisfaz todos os predicados do filtro.
    ///
    /// Um filtro vazio corresponde a `true`, não restringindo a busca.
    pub fn matches(&self, metadata: &DocumentMetadata) -> bool {
        self.and.iter().all(|filter| match filter {
            Filter::Eq { key, value } => metadata.get(key).is_some_and(|v| v == value),
            Filter::In { key, values } => metadata.get(key).is_some_and(|v| values.contains(v)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn eq_matches_when_value_equal() {
        let mut meta = DocumentMetadata::new();
        meta.insert("repository".into(), json!("gerax"));
        let filter = VectorFilter::and([Filter::eq("repository", "gerax")]);
        assert!(filter.matches(&meta));
    }

    #[test]
    fn eq_matches_when_value_differs() {
        let mut meta = DocumentMetadata::new();
        meta.insert("language".into(), json!("rust"));
        let filter = VectorFilter::and([Filter::eq("language", "python")]);
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn in_matches_any_value() {
        let mut meta = DocumentMetadata::new();
        meta.insert("branch".into(), json!("main"));
        let filter = VectorFilter::and([Filter::in_list(
            "branch",
            vec![json!("develop"), json!("main")],
        )]);
        assert!(filter.matches(&meta));
    }

    #[test]
    fn and_requires_all() {
        let mut meta = DocumentMetadata::new();
        meta.insert("repository".into(), json!("gerax"));
        meta.insert("language".into(), json!("python"));
        let filter = VectorFilter::and([
            Filter::eq("repository", "gerax"),
            Filter::eq("language", "rust"),
        ]);
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn missing_key_does_not_match() {
        let meta = DocumentMetadata::new();
        let filter = VectorFilter::and([Filter::eq("repository", "gerax")]);
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn empty_filter_matches_everything() {
        let meta = DocumentMetadata::new();
        let filter = VectorFilter::default();
        assert!(filter.matches(&meta));
    }
}
