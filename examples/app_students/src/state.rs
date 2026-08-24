use std::sync::Arc;

use gerax_app::PostgresConnection;

use crate::services::{ClassService, EnrollmentService, StudentService, TeacherService};

pub struct AppState {
    pub students: Arc<StudentService>,
    pub teachers: Arc<TeacherService>,
    pub classes: Arc<ClassService>,
    pub enrollments: Arc<EnrollmentService>,
}

impl AppState {
    pub fn new(db: Arc<PostgresConnection>) -> Self {
        Self {
            students: Arc::new(StudentService::new(db.clone())),
            teachers: Arc::new(TeacherService::new(db.clone())),
            classes: Arc::new(ClassService::new(db.clone())),
            enrollments: Arc::new(EnrollmentService::new(db)),
        }
    }
}
