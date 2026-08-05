use crate::GraphqlError;

/// Controlador de introspection GraphQL.
///
/// Permite habilitar ou desabilitar a introspection
/// para prevenir exposição indevida do schema.
pub struct IntrospectionController {
    enabled: bool,
}

impl IntrospectionController {
    /// Cria um novo controlador de introspection habilitado por padrão.
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Cria um novo controlador de introspection com o estado inicial.
    pub fn with_state(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Retorna se a introspection está habilitada.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Habilita a introspection.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Desabilita a introspection.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Verifica se uma query é de introspection.
    ///
    /// Queries de introspection contêm as palavras-chave
    /// `__schema` ou `__type`.
    pub fn is_introspection_query(&self, query: &str) -> bool {
        query.contains("__schema") || query.contains("__type")
    }

    /// Valida se a introspection está permitida para a query.
    ///
    /// Retorna um erro se a introspection estiver desabilitada
    /// e a query tentar acessar o schema interno.
    pub fn validate(&self, query: &str) -> Result<(), GraphqlError> {
        if self.is_introspection_query(query) && !self.enabled {
            return Err(GraphqlError::Validation(
                "introspection is disabled".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for IntrospectionController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::IntrospectionController;
    use crate::GraphqlError;

    #[test]
    fn disabled_controller_rejects_introspection_only() {
        let controller = IntrospectionController::with_state(false);

        assert!(matches!(
            controller.validate("{ __schema { queryType { name } } }"),
            Err(GraphqlError::Validation(_))
        ));
        assert!(controller.validate("{ viewer { id } }").is_ok());
    }
}
