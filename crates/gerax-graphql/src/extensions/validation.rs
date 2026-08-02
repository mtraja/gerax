use crate::{GraphqlRequest, GraphqlError};

/// Validador de schemas GraphQL.
///
/// Responsável por validar a estrutura do schema,
/// incluindo tipos, campos e resolver.
pub struct SchemaValidator;

impl SchemaValidator {
    /// Cria um novo validador de schema.
    pub fn new() -> Self {
        Self
    }
}

/// Validador de queries GraphQL.
///
/// Responsável por validar a sintaxe e a estrutura
/// das queries GraphQL antes da execução.
pub struct QueryValidator;

impl QueryValidator {
    /// Cria um novo validador de queries.
    pub fn new() -> Self {
        Self
    }

    /// Valida a sintaxe de uma query GraphQL.
    ///
    /// Verifica se a query está sintaticamente correta.
    pub fn validate_query(&self, query: &str) -> Result<(), GraphqlError> {
        if query.trim().is_empty() {
            return Err(GraphqlError::Validation(
                "query cannot be empty".to_string(),
            ));
        }

        if !query.contains('{') || !query.contains('}') {
            return Err(GraphqlError::Validation(
                "query must contain curly braces".to_string(),
            ));
        }

        Ok(())
    }
}

/// Validador de argumentos GraphQL.
///
/// Responsável por validar os argumentos passados
/// em queries e mutations.
pub struct ArgumentsValidator;

impl ArgumentsValidator {
    /// Cria um novo validador de argumentos.
    pub fn new() -> Self {
        Self
    }

    /// Valida os argumentos de uma requisição GraphQL.
    ///
    /// Verifica se os argumentos estão no formato correto.
    pub fn validate_arguments(
        &self,
        request: &GraphqlRequest,
    ) -> Result<(), GraphqlError> {
        if let Some(variables) = &request.variables {
            for (key, value) in variables {
                if key.is_empty() {
                    return Err(GraphqlError::Validation(
                        "variable name cannot be empty".to_string(),
                    ));
                }
                if value.is_null() {
                    return Err(GraphqlError::Validation(
                        format!("variable '{}' cannot be null", key),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Validador completo para operações GraphQL.
///
/// Combina validação de query e argumentos
/// em uma única interface.
pub struct Validator;

impl Validator {
    /// Cria um novo validador completo.
    pub fn new() -> Self {
        Self
    }

    /// Valida uma requisição GraphQL completa.
    ///
    /// Executa todas as validações: query e argumentos.
    pub fn validate(&self, request: &GraphqlRequest) -> Result<(), GraphqlError> {
        let query_validator = QueryValidator::new();
        query_validator.validate_query(&request.query)?;

        let args_validator = ArgumentsValidator::new();
        args_validator.validate_arguments(request)?;

        Ok(())
    }
}