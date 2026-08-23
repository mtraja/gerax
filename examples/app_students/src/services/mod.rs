use std::sync::Arc;

use gerax_app::{DbError, PostgresConnection, PostgresRepository, Repository};
use gerax_core::Entity;

use crate::models::{CreateStudentInput, Student, UpdateStudentInput};

pub struct StudentService {
    db: Arc<PostgresConnection>,
}

impl StudentService {
    pub fn new(db: Arc<PostgresConnection>) -> Self {
        Self { db }
    }

    fn repo(&self) -> PostgresRepository<Student> {
        PostgresRepository::<Student>::new(self.db.clone())
    }

    pub async fn list(&self) -> Result<Vec<Student>, DbError> {
        self.repo().find_all().await
    }

    pub async fn get(&self, id: &str) -> Result<Option<Student>, DbError> {
        self.repo().find_by_id(id).await
    }

    pub async fn create(&self, input: CreateStudentInput) -> Result<Student, DbError> {
        let student = Student {
            id: None,
            name: input.name,
            email: input.email,
        };
        self.repo().insert(student).await
    }

    pub async fn update(&self, id: &str, input: UpdateStudentInput) -> Result<Student, DbError> {
        let repo = self.repo();
        let existing = repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DbError::not_found(id))?;

        let student = Student {
            id: existing.id(),
            name: input.name,
            email: input.email,
        };
        repo.update(student).await?;

        repo.find_by_id(id)
            .await?
            .ok_or_else(|| DbError::not_found(id))
    }

    pub async fn delete(&self, id: &str) -> Result<(), DbError> {
        self.repo().delete(id).await
    }
}
