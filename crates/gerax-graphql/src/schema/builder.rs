use gerax_core::Entity;

/// Builder para construção de schema GraphQL.
///
/// Permite registrar entidades do `gerax-core` para gerar tipos e resolvers automaticamente.
pub struct SchemaBuilder {
    entities: Vec<String>,
}

impl SchemaBuilder {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Registra uma entidade no schema.
    ///
    /// A entidade deve implementar `gerax_core::Entity`.
    /// O nome da coleção será usado como tipo no GraphQL.
    pub fn entity<T: Entity + Send + Sync + 'static>(mut self) -> Self {
        self.entities.push(T::collection_name().to_string());
        self
    }
}
