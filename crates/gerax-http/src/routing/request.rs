use std::sync::Arc;

#[derive(Clone)]
pub struct Request<State> {
    pub state: Arc<State>,
    pub path: String,
    pub body: Vec<u8>,
}
