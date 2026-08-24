use std::sync::Arc;

use gerax_app::{DbError, PostgresConnection, PostgresRepository, Repository};

use crate::models::{
    Class, CreateClassInput, CreateEnrollmentInput, CreateStudentInput, CreateTeacherInput,
    Enrollment, Student, UpdateClassInput, UpdateStudentInput, UpdateTeacherInput, Teacher,
};

pub trait CrudEntity<C, U> {
    fn new(input: C) -> Self;
    fn apply_update(&mut self, input: U);
}

impl CrudEntity<CreateStudentInput, UpdateStudentInput> for Student {
    fn new(input: CreateStudentInput) -> Self {
        Self { id: None, name: input.name, email: input.email }
    }

    fn apply_update(&mut self, input: UpdateStudentInput) {
        self.name = input.name;
        self.email = input.email;
    }
}

impl CrudEntity<CreateTeacherInput, UpdateTeacherInput> for Teacher {
    fn new(input: CreateTeacherInput) -> Self {
        Self { id: None, name: input.name, email: input.email }
    }

    fn apply_update(&mut self, input: UpdateTeacherInput) {
        self.name = input.name;
        self.email = input.email;
    }
}

impl CrudEntity<CreateClassInput, UpdateClassInput> for Class {
    fn new(input: CreateClassInput) -> Self {
        Self { id: None, name: input.name, teacher_id: input.teacher_id }
    }

    fn apply_update(&mut self, input: UpdateClassInput) {
        self.name = input.name;
        self.teacher_id = input.teacher_id;
    }
}

macro_rules! crud_service {
    ($service:ident, $entity:ty, $create:ty, $update:ty) => {
        pub struct $service {
            db: Arc<PostgresConnection>,
        }

        impl $service {
            pub fn new(db: Arc<PostgresConnection>) -> Self {
                Self { db }
            }

            fn repo(&self) -> PostgresRepository<$entity> {
                PostgresRepository::<$entity>::new(self.db.clone())
            }

            pub async fn list(&self) -> Result<Vec<$entity>, DbError> {
                self.repo().find_all().await
            }

            pub async fn get(&self, id: &str) -> Result<Option<$entity>, DbError> {
                self.repo().find_by_id(id).await
            }

            pub async fn create(&self, input: $create) -> Result<$entity, DbError> {
                let entity = <$entity>::new(input);
                self.repo().insert(entity).await
            }

            pub async fn update(&self, id: &str, input: $update) -> Result<$entity, DbError> {
                let repo = self.repo();
                let mut entity = repo
                    .find_by_id(id)
                    .await?
                    .ok_or_else(|| DbError::not_found(id))?;

                entity.apply_update(input);
                repo.update(entity).await?;

                repo.find_by_id(id)
                    .await?
                    .ok_or_else(|| DbError::not_found(id))
            }

            pub async fn delete(&self, id: &str) -> Result<(), DbError> {
                self.repo().delete(id).await
            }
        }
    };
}

crud_service!(StudentService, Student, CreateStudentInput, UpdateStudentInput);
crud_service!(TeacherService, Teacher, CreateTeacherInput, UpdateTeacherInput);
crud_service!(ClassService, Class, CreateClassInput, UpdateClassInput);

pub struct EnrollmentService {
    db: Arc<PostgresConnection>,
}

impl EnrollmentService {
    pub fn new(db: Arc<PostgresConnection>) -> Self {
        Self { db }
    }

    fn repo(&self) -> PostgresRepository<Enrollment> {
        PostgresRepository::<Enrollment>::new(self.db.clone())
    }

    pub async fn list(&self) -> Result<Vec<Enrollment>, DbError> {
        self.repo().find_all().await
    }

    pub async fn get(&self, id: &str) -> Result<Option<Enrollment>, DbError> {
        self.repo().find_by_id(id).await
    }

    pub async fn create(&self, input: CreateEnrollmentInput) -> Result<Enrollment, DbError> {
        let enrollment = Enrollment {
            id: None,
            student_id: input.student_id,
            class_id: input.class_id,
        };
        self.repo().insert(enrollment).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), DbError> {
        self.repo().delete(id).await
    }
}
