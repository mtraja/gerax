use crate::GraphqlError;

/// Limite configurável de complexidade para queries GraphQL.
///
/// A complexidade é calculada com base no número de campos selecionados
/// na query, incluindo campos aninhados.
pub struct ComplexityLimiter {
    max_complexity: usize,
}

impl ComplexityLimiter {
    /// Cria um novo limitador de complexidade com o limite máximo.
    pub fn new(max_complexity: usize) -> Self {
        Self { max_complexity }
    }

    /// Retorna o limite máximo de complexidade.
    pub fn max_complexity(&self) -> usize {
        self.max_complexity
    }

    /// Calcula a complexidade de uma query.
    ///
    /// A complexidade é estimada pelo número de seletores de campo
    /// encontrados na query.
    pub fn calculate_complexity(&self, query: &str) -> usize {
        let mut complexity = 0;

        for token in query.split_whitespace() {
            let trimmed = token.trim_matches(|c| {
                c == '{' || c == '}' || c == '(' || c == ')' || c == ',' || c == ':'
            });

            if trimmed.is_empty() {
                continue;
            }

            if trimmed == "{" || trimmed == "}" {
                continue;
            }

            if trimmed.starts_with("__") {
                continue;
            }

            if matches!(
                trimmed,
                "query"
                    | "mutation"
                    | "subscription"
                    | "fragment"
                    | "on"
                    | "True"
                    | "False"
                    | "Null"
                    | "true"
                    | "false"
                    | "null"
            ) {
                continue;
            }

            if trimmed
                .chars()
                .all(|c| c.is_ascii_punctuation() || c.is_ascii_digit())
            {
                continue;
            }

            complexity += 1;
        }

        complexity
    }

    /// Verifica se a complexidade da query excede o limite.
    pub fn check(&self, query: &str) -> Result<(), GraphqlError> {
        let complexity = self.calculate_complexity(query);
        if complexity > self.max_complexity {
            return Err(GraphqlError::ComplexityExceeded(format!(
                "query complexity {} exceeds limit {}",
                complexity, self.max_complexity
            )));
        }
        Ok(())
    }
}

/// Limite configurável de profundidade para queries GraphQL.
///
/// A profundidade é calculada pelo número de níveis
/// de aninhamento na query.
pub struct DepthLimiter {
    max_depth: usize,
}

impl DepthLimiter {
    /// Cria um novo limitador de profundidade com o limite máximo.
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Retorna o limite máximo de profundidade.
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Calcula a profundidade de uma query.
    ///
    /// A profundidade é estimada pelo número de chaves
    /// de abertura na query.
    pub fn calculate_depth(&self, query: &str) -> usize {
        query.chars().filter(|c| *c == '{').count()
    }

    /// Verifica se a profundidade da query excede o limite.
    pub fn check(&self, query: &str) -> Result<(), GraphqlError> {
        let depth = self.calculate_depth(query);
        if depth > self.max_depth {
            return Err(GraphqlError::DepthExceeded(format!(
                "query depth {} exceeds limit {}",
                depth, self.max_depth
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ComplexityLimiter, DepthLimiter};
    use crate::GraphqlError;

    #[test]
    fn complexity_limiter_rejects_queries_above_its_limit() {
        let limiter = ComplexityLimiter::new(2);

        assert!(matches!(
            limiter.check("{ firstField secondField thirdField }"),
            Err(GraphqlError::ComplexityExceeded(_))
        ));
    }

    #[test]
    fn complexity_limiter_counts_nested_fields() {
        let limiter = ComplexityLimiter::new(2);

        assert!(matches!(
            limiter.check("{ user { name email } }"),
            Err(GraphqlError::ComplexityExceeded(_))
        ));
    }

    #[test]
    fn complexity_limiter_ignores_introspection_and_keywords() {
        let limiter = ComplexityLimiter::new(3);

        assert!(limiter.check("{ __schema { queryType { name } } }").is_ok());
        assert!(limiter.check("query User { user { id } }").is_ok());
    }

    #[test]
    fn depth_limiter_accepts_and_rejects_expected_depths() {
        let limiter = DepthLimiter::new(2);

        assert!(limiter.check("{ user { id } }").is_ok());
        assert!(matches!(
            limiter.check("{ user { profile { id } } }"),
            Err(GraphqlError::DepthExceeded(_))
        ));
    }
}
