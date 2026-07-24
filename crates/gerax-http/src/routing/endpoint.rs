use std::sync::Arc;

use super::{Handler, HttpMethod};

pub struct Route<State> {
    pub method: HttpMethod,
    pub path: String,
    pub handler: Arc<dyn Handler<State>>,
}
