use gerax_core::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Student {
    pub id: Option<String>,
    pub name: String,
    pub email: String,
}

impl Entity for Student {
    fn collection_name() -> &'static str {
        "students"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateStudentInput {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStudentInput {
    pub name: String,
    pub email: String,
}
