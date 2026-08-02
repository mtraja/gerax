use gerax_core::Entity;

/// Builder para geração de especificação OpenAPI a partir de entidades Gerax.
///
/// Registra entidades e produz definições OpenAPI (Swagger) para documentação
/// automática da API.
pub struct OpenApiBuilder {
    entities: Vec<String>,
}

impl OpenApiBuilder {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Registra uma entidade no schema OpenAPI.
    ///
    /// A entidade deve implementar `gerax_core::Entity`.
    /// O nome da coleção será usado como nome do endpoint e do schema.
    pub fn entity<T: Entity + Send + Sync + 'static>(mut self) -> Self {
        self.entities.push(T::collection_name().to_string());
        self
    }
}