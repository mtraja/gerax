use std::sync::Arc;

use gerax_app::PostgresConnection;

use crate::services::StudentService;

pub struct AppState {
    pub service: Arc<StudentService>,
}

impl AppState {
    pub fn new(db: Arc<PostgresConnection>) -> Self {
        Self {
            service: Arc::new(StudentService::new(db)),
        }
    }
}
