use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::{DbError, Repository};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;


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
            "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, data TEXT)",
            self.table_name()
        );
        self.connection.client().execute(&query, &[])
            .await
            .map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}

#[async_trait]
impl<T> Repository<T> for PostgresRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, DbError> {
        let query = format!("SELECT data FROM {} WHERE id = $1", self.table_name());
        let row = self.connection.client().query_opt(&query, &[&id])
            .await
            .map_err(|e| DbError::connection(e))?;

        match row {
            Some(row) => {
                let data: String = row.get(0);
                let entity = serde_json::from_str(&data)
                    .map_err(|e| DbError::serialization(e))?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<T>, DbError> {
        let query = format!("SELECT data FROM {}", self.table_name());
        let rows = self.connection.client().query(&query, &[])
            .await
            .map_err(|e| DbError::connection(e))?;

        let mut entities = Vec::new();
        for row in rows {
            let data: String = row.get(0);
            let entity = serde_json::from_str(&data)
                .map_err(|e| DbError::serialization(e))?;
            entities.push(entity);
        }
        Ok(entities)
    }

    async fn insert(&self, mut entity: T) -> Result<T, DbError> {
        let id = entity.id().unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string()
        });
        entity.set_id(id.clone());

        let data = serde_json::to_string(&entity)
            .map_err(|e| DbError::serialization(e))?;

        let query = format!(
            "INSERT INTO {} (id, data) VALUES ($1, $2) RETURNING data",
            self.table_name()
        );
        let row = self.connection.client().query_one(&query, &[&id, &data])
            .await
            .map_err(|e| DbError::connection(e))?;

        let returned_data: String = row.get(0);
        let returned_entity = serde_json::from_str(&returned_data)
            .map_err(|e| DbError::serialization(e))?;
        Ok(returned_entity)
    }

    async fn update(&self, entity: T) -> Result<(), DbError> {
        let id = entity
            .id()
            .ok_or_else(|| DbError::not_found("missing id"))?;
        let data = serde_json::to_string(&entity)
            .map_err(|e| DbError::serialization(e))?;

        let query = format!("UPDATE {} SET data = $1 WHERE id = $2", self.table_name());
        let rows_affected = self.connection.client().execute(&query, &[&data, &id])
            .await
            .map_err(|e| DbError::connection(e))?;

        if rows_affected == 0 {
            return Err(DbError::not_found(id));
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DbError> {
        let query = format!("DELETE FROM {} WHERE id = $1", self.table_name());
        let rows_affected = self.connection.client().execute(&query, &[&id])
            .await
            .map_err(|e| DbError::connection(e))?;

        if rows_affected == 0 {
            return Err(DbError::not_found(id));
        }
        Ok(())
    }
}
