use gerax_app::Router;

use crate::handlers::{
    create_student, delete_student, get_student, list_students, update_student,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .get("/students", list_students)
        .post("/students", create_student)
        .get("/students/:id", get_student)
        .put("/students/:id", update_student)
        .delete("/students/:id", delete_student)
}
