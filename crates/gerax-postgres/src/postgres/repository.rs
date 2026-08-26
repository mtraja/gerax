use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::{DbError, Repository};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::marker::PhantomData;
use uuid::Uuid;

use crate::postgres::PostgresConnection;

pub struct PostgresRepository<T> {
    connection: std::sync::Arc<PostgresConnection>,
    _marker: PhantomData<T>,
}

impl<T> PostgresRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(connection: std::sync::Arc<PostgresConnection>) -> Self {
        Self {
            connection,
            _marker: PhantomData,
        }
    }

    pub fn connection(&self) -> &std::sync::Arc<PostgresConnection> {
        &self.connection
    }

    fn table_name(&self) -> &str {
        T::collection_name()
    }

    pub async fn create_table(&self) -> Result<(), DbError> {
        let query = format!(
            "CREATE TABLE IF NOT EXISTS {} (id UUID PRIMARY KEY, data JSONB)",
            self.table_name()
        );
        sqlx::query(&query)
            .execute(self.connection.client())
            .await
            .map_err(DbError::connection)?;
        Ok(())
    }
}

#[async_trait]
impl<T> Repository<T> for PostgresRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, DbError> {
        let uuid = Uuid::parse_str(id).map_err(DbError::configuration)?;

        let query = format!("SELECT data FROM {} WHERE id = $1", self.table_name());
        let row = sqlx::query(&query)
            .bind(uuid)
            .fetch_optional(self.connection.client())
            .await
            .map_err(DbError::connection)?;

        match row {
            Some(row) => {
                let data: String = row.get("data");
                let entity = serde_json::from_str(&data)
                    .map_err(DbError::serialization)?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<T>, DbError> {
        let query = format!("SELECT data FROM {}", self.table_name());
        let rows = sqlx::query(&query)
            .fetch_all(self.connection.client())
            .await
            .map_err(DbError::connection)?;

        let mut entities = Vec::new();
        for row in rows {
            let data: String = row.get("data");
            let entity = serde_json::from_str(&data)
                .map_err(DbError::serialization)?;
            entities.push(entity);
        }
        Ok(entities)
    }

    async fn insert(&self, mut entity: T) -> Result<T, DbError> {
        let uuid = Uuid::new_v4();
        entity.set_id(uuid.to_string());

        let data = serde_json::to_string(&entity)
            .map_err(DbError::serialization)?;

        let query = format!(
            "INSERT INTO {} (id, data) VALUES ($1, $2) RETURNING data",
            self.table_name()
        );
        let row = sqlx::query(&query)
            .bind(uuid)
            .bind(&data)
            .fetch_one(self.connection.client())
            .await
            .map_err(DbError::connection)?;

        let returned_data: String = row.get("data");
        let returned_entity = serde_json::from_str(&returned_data)
            .map_err(DbError::serialization)?;
        Ok(returned_entity)
    }

    async fn update(&self, entity: T) -> Result<(), DbError> {
        let id = entity
            .id()
            .ok_or_else(|| DbError::not_found("missing id"))?;
        let uuid = Uuid::parse_str(&id).map_err(DbError::configuration)?;
        let data = serde_json::to_string(&entity)
            .map_err(DbError::serialization)?;

        let query = format!("UPDATE {} SET data = $1 WHERE id = $2", self.table_name());
        let rows_affected = sqlx::query(&query)
            .bind(&data)
            .bind(uuid)
            .execute(self.connection.client())
            .await
            .map_err(DbError::connection)?
            .rows_affected();

        if rows_affected == 0 {
            return Err(DbError::not_found(id));
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DbError> {
        let uuid = Uuid::parse_str(id).map_err(DbError::configuration)?;

        let query = format!("DELETE FROM {} WHERE id = $1", self.table_name());
        let rows_affected = sqlx::query(&query)
            .bind(uuid)
            .execute(self.connection.client())
            .await
            .map_err(DbError::connection)?
            .rows_affected();

        if rows_affected == 0 {
            return Err(DbError::not_found(id));
        }
        Ok(())
    }
}
