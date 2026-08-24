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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Teacher {
    pub id: Option<String>,
    pub name: String,
    pub email: String,
}

impl Entity for Teacher {
    fn collection_name() -> &'static str {
        "teachers"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTeacherInput {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeacherInput {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub id: Option<String>,
    pub name: String,
    pub teacher_id: String,
}

impl Entity for Class {
    fn collection_name() -> &'static str {
        "classes"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateClassInput {
    pub name: String,
    pub teacher_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClassInput {
    pub name: String,
    pub teacher_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: Option<String>,
    pub student_id: String,
    pub class_id: String,
}

impl Entity for Enrollment {
    fn collection_name() -> &'static str {
        "enrollments"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateEnrollmentInput {
    pub student_id: String,
    pub class_id: String,
}
