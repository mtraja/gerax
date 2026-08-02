use gerax_core::Entity;

/// Builder para geração de schema Cap'n Proto a partir de entidades Gerax.
///
/// Registra entidades e produz definições `.capnp` ou builders
/// de servidor/cliente tipados.
pub struct CapnpBuilder {
    entities: Vec<String>,
}

impl CapnpBuilder {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    /// Registra uma entidade no schema.
    ///
    /// A entidade deve implementar `gerax_core::Entity`.
    /// O nome da coleção será usado como nome do tipo/mensagem no Cap'n Proto.
    pub fn entity<T: Entity + Send + Sync + 'static>(mut self) -> Self {
        self.entities.push(T::collection_name().to_string());
        self
    }
}