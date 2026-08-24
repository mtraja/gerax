use gerax_app::Router;

use crate::handlers::{
    create_class, create_enrollment, create_student, create_teacher, delete_class,
    delete_enrollment, delete_student, delete_teacher, get_class, get_enrollment, get_student,
    get_teacher, list_classes, list_enrollments, list_students, list_teachers, update_class,
    update_student, update_teacher,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .get("/students", list_students)
        .post("/students", create_student)
        .get("/students/:id", get_student)
        .put("/students/:id", update_student)
        .delete("/students/:id", delete_student)
        .get("/teachers", list_teachers)
        .post("/teachers", create_teacher)
        .get("/teachers/:id", get_teacher)
        .put("/teachers/:id", update_teacher)
        .delete("/teachers/:id", delete_teacher)
        .get("/classes", list_classes)
        .post("/classes", create_class)
        .get("/classes/:id", get_class)
        .put("/classes/:id", update_class)
        .delete("/classes/:id", delete_class)
        .get("/enrollments", list_enrollments)
        .post("/enrollments", create_enrollment)
        .get("/enrollments/:id", get_enrollment)
        .delete("/enrollments/:id", delete_enrollment)
}
