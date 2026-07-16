use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::{Connection, DbError, Repository};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tokio_postgres::{Client, NoTls};

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub uri: String,
    pub database: String,
}

impl PostgresConfig {
    pub fn from_env() -> Result<Self, DbError> {
        dotenv::dotenv().ok();
        let uri = std::env::var("DATABASE_URL")
            .map_err(|_| DbError::connection("DATABASE_URL not set in environment"))?;
        let database = std::env::var("DATABASE_NAME")
            .map_err(|_| DbError::connection("DATABASE_NAME not set in environment"))?;
        Ok(Self { uri, database })
    }
}

pub struct PostgresRepository<T> {
    client: Client,
    _marker: PhantomData<T>,
}

impl<T> PostgresRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    fn table_name(&self) -> &str {
        T::collection_name()
    }

    pub async fn create_table(&self) -> Result<(), DbError> {
        let query = format!(
            "CREATE TABLE IF NOT EXISTS {} (id TEXT PRIMARY KEY, data TEXT)",
            self.table_name()
        );
        self.client.execute(&query, &[])
            .await
            .map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}

#[async_trait]
impl<T> Connection for PostgresRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = PostgresConfig::from_env()?;
        let (client, connection) = tokio_postgres::connect(&config.uri, NoTls)
            .await
            .map_err(|e| DbError::connection(e))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        Ok(Self {
            client,
            _marker: PhantomData,
        })
    }

    async fn ping(&self) -> Result<(), DbError> {
        self.client.execute("SELECT 1", &[])
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
        let row = self.client.query_opt(&query, &[&id])
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
        let rows = self.client.query(&query, &[])
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
        let row = self.client.query_one(&query, &[&id, &data])
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
        let rows_affected = self.client.execute(&query, &[&data, &id])
            .await
            .map_err(|e| DbError::connection(e))?;

        if rows_affected == 0 {
            return Err(DbError::not_found(id));
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DbError> {
        let query = format!("DELETE FROM {} WHERE id = $1", self.table_name());
        let rows_affected = self.client.execute(&query, &[&id])
            .await
            .map_err(|e| DbError::connection(e))?;

        if rows_affected == 0 {
            return Err(DbError::not_found(id));
        }
        Ok(())
    }
}
