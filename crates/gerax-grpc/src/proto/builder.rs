use gerax_core::Entity;

pub struct ProtoBuilder {
    entities: Vec<String>,
}

impl ProtoBuilder {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    pub fn entity<T: Entity + Send + Sync + 'static>(mut self) -> Self {
        self.entities.push(T::collection_name().to_string());
        self
    }
}
