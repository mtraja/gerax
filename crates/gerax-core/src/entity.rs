use serde::{Deserialize, Serialize};

pub trait Entity: Serialize + for<'de> Deserialize<'de> + Send + Sync + Unpin + Clone + 'static {
    fn collection_name() -> &'static str;
    fn id(&self) -> Option<String>;
    fn set_id(&mut self, id: String);
}